//! The native pet window: a transparent, always-on-top layered window painted
//! with tiny-skia. No WebView2 is created for the pet.
//!
//! The window owns its message loop on a dedicated thread. It polls the cursor
//! (the engine is clock-free; the pointer is the only external input), hit-tests
//! the silhouette to toggle `WS_EX_TRANSPARENT`, and lets the OS drag the window
//! when the body is grabbed.

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Once, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tiny_skia::Pixmap;
use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetMonitorInfoW,
    MonitorFromPoint, MonitorFromWindow, SelectObject, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ, MONITORINFO,
    MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows_sys::Win32::System::SystemInformation::GetTickCount64;
use windows_sys::Win32::System::Threading::GetSystemTimes;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetLastInputInfo, ReleaseCapture, LASTINPUTINFO,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetCursorPos, GetWindowLongPtrW, GetWindowRect, PeekMessageW, PostMessageW,
    PostQuitMessage, RegisterClassW, SendMessageW, SetForegroundWindow, SetWindowLongPtrW,
    SetWindowPos, ShowWindow, TrackPopupMenu, TranslateMessage, UpdateLayeredWindow, GWL_EXSTYLE,
    MF_SEPARATOR, MF_STRING, MSG, PM_REMOVE, SWP_NOACTIVATE, SWP_NOZORDER, SW_HIDE, SW_SHOW,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, ULW_ALPHA, WM_APP, WM_CLOSE, WM_DESTROY, WM_LBUTTONDOWN,
    WM_NCLBUTTONDOWN, WM_NULL, WM_QUIT, WM_RBUTTONUP, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

use crate::engine::{catalogue, BotEngine, ExprId, Look, ShapeId, StateId};
use crate::render::{self, DEMI};

const R: f64 = 100.0;
const YAW_MAX: f64 = 18.0;
const PITCH_BASE: f64 = 10.0;
const PITCH_MAX: f64 = 13.0;
const WAKE_MS: u64 = 900;
const IDLE_MS: u64 = 66;
const BUSY_MS: u64 = 15;
const HTCAPTION: usize = 2;
/// Two clicks within this window count as a double-click.
const DOUBLE_CLICK_MS: u64 = 320;
/// Idle minutes from the config as a duration; `0` means never lie down.
///
/// The default lives in `Config::default` (`lib.rs`), which is what a config
/// written before the field existed falls back to.
fn sleep_after_ms(minutes: u32) -> Option<u64> {
    if minutes == 0 {
        None
    } else {
        Some(u64::from(minutes) * 60_000)
    }
}

/// How long the whole session has gone without keyboard or mouse input, in ms.
///
/// `GetLastInputInfo` is the screensaver clock: it covers the keyboard as well as
/// the mouse, so typing counts even though the pet itself only ever sees the
/// cursor. It shares a clock with `GetTickCount64`, so the subtraction is exact and
/// both wrap together.
fn system_idle_ms() -> u64 {
    let mut info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    if unsafe { GetLastInputInfo(&mut info) } == 0 {
        // No input information: stay conservatively awake rather than put the pet
        // to sleep on a guess.
        return 0;
    }
    let now = unsafe { GetTickCount64() } as u32;
    u64::from(now.wrapping_sub(info.dwTime))
}

/// Whether the pet should lie down, given the session's idle time.
///
/// Pure, so the decision is unit-tested instead of needing a real desktop.
/// `in_window` guards a pointer parked on the pet: the session reads as idle, but
/// the pet must stay awake, or it would sleep, wake itself with a yawn, and repeat.
fn should_sleep(idle_ms: u64, after: Option<u64>, in_window: bool, emote_active: bool) -> bool {
    !in_window && !emote_active && after.is_some_and(|ms| idle_ms >= ms)
}

/* ------------------------------------------------------------------- mood */

/// How often the CPU is sampled, and how often the battery is re-read.
const LOAD_SAMPLE_MS: u64 = 1000;
const POWER_SAMPLE_MS: u64 = 5000;
/// Weight of a new CPU sample in the running average. Low enough that one busy
/// frame does not change the face, high enough to react within a few seconds.
const LOAD_EMA: f64 = 0.4;

/// Load has to cross one line to earn a mood and drop below a lower one to lose
/// it, so a machine sitting on the boundary does not flicker the face.
const BUSY_ENTER: f64 = 0.45;
const BUSY_EXIT: f64 = 0.30;
const PEGGED_ENTER: f64 = 0.85;
const PEGGED_EXIT: f64 = 0.70;
/// On battery below this, the pet gets drowsy.
const BATTERY_LOW_PERCENT: u8 = 20;

/// What the machine is doing, as the pet reads it. Four coarse states, because
/// that is all a face can carry — this is a feel, not a gauge.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mood {
    Calm,
    Busy,
    Pegged,
    LowBattery,
}

/// The mood for a load, with hysteresis against `current`. Pure, so the mapping
/// is unit-tested instead of eyeballed against a running machine.
fn mood_for(load: f64, on_battery: bool, battery_percent: u8, current: Mood) -> Mood {
    // Battery outranks load: it is the one that wants an action from you.
    if on_battery && battery_percent <= BATTERY_LOW_PERCENT {
        return Mood::LowBattery;
    }
    if load >= PEGGED_ENTER || (current == Mood::Pegged && load >= PEGGED_EXIT) {
        return Mood::Pegged;
    }
    if load >= BUSY_ENTER || (current == Mood::Busy && load >= BUSY_EXIT) {
        return Mood::Busy;
    }
    Mood::Calm
}

/// The face a mood wears. `Calm` hands the pet back its own expression, so a
/// quiet machine always looks the way you configured it.
fn mood_expression(mood: Mood, user: ExprId) -> ExprId {
    match mood {
        Mood::Calm => user,
        Mood::Busy => ExprId::Attentif,
        Mood::Pegged => ExprId::Effraye,
        Mood::LowBattery => ExprId::Somnolent,
    }
}

/// Busy fraction between two `GetSystemTimes` samples.
///
/// `kernel` already contains `idle`, so the total is `kernel + user` and the busy
/// part is that minus idle. Pure: the caller owns the clock and the sampling.
fn busy_fraction(prev: (u64, u64, u64), now: (u64, u64, u64)) -> Option<f64> {
    let idle = now.0.saturating_sub(prev.0);
    let total = now.1.saturating_sub(prev.1) + now.2.saturating_sub(prev.2);
    if total == 0 {
        return None;
    }
    Some(1.0 - idle as f64 / total as f64)
}

fn filetime_ms(t: FILETIME) -> u64 {
    ((t.dwHighDateTime as u64) << 32) | t.dwLowDateTime as u64
}

/// System times as (idle, kernel, user) hundred-nanosecond ticks.
fn system_times() -> (u64, u64, u64) {
    let mut idle = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut kernel = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut user = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
    (filetime_ms(idle), filetime_ms(kernel), filetime_ms(user))
}

/// Whether the machine is on battery, and how full it is.
fn power_status() -> (bool, u8) {
    let mut s: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    if unsafe { GetSystemPowerStatus(&mut s) } == 0 {
        return (false, 100);
    }
    // 255 is the API's "unknown" for both, which is what a desktop on mains
    // reports; treat it as plugged in and full rather than as a dead battery.
    let on_battery = s.ACLineStatus == 0;
    let percent = if s.BatteryLifePercent == 255 {
        100
    } else {
        s.BatteryLifePercent
    };
    (on_battery, percent)
}

/// CPU load over time, so the mood sees an average and not a single sample.
#[derive(Default)]
struct LoadSampler {
    prev: Option<(u64, u64, u64)>,
    sampled_at: u64,
    ema: Option<f64>,
}

impl LoadSampler {
    /// The smoothed busy fraction, resampled at most once per `LOAD_SAMPLE_MS`.
    fn load(&mut self, now_ms: u64) -> f64 {
        if self.prev.is_none() || now_ms.saturating_sub(self.sampled_at) >= LOAD_SAMPLE_MS {
            self.sampled_at = now_ms;
            let now = system_times();
            if let Some(prev) = self.prev {
                if let Some(load) = busy_fraction(prev, now) {
                    self.ema = Some(match self.ema {
                        None => load,
                        Some(e) => e + LOAD_EMA * (load - e),
                    });
                }
            }
            self.prev = Some(now);
        }
        self.ema.unwrap_or(0.0)
    }
}

/// Tick of the previous click, and reactions asked for by the window procedure.
/// The engine lives on the pet thread, so the procedure only raises a flag.
static LAST_CLICK: AtomicU64 = AtomicU64::new(0);
static WINK_REQUESTED: AtomicBool = AtomicBool::new(false);
static WIDE_REQUESTED: AtomicBool = AtomicBool::new(false);
/// Raised by the pet's own right-click menu.
static EMOTE_REQUESTED: AtomicBool = AtomicBool::new(false);
/// Raised by a second launch, which asks this pet to come back on screen.
static SHOW_REQUESTED: AtomicBool = AtomicBool::new(false);

/// The pet window's class; a second launch also looks for it by name.
pub const PET_CLASS: &str = "DotNativePet";

/// Our own message: a second Dot posts it to ask this pet to show itself.
pub const WM_APP_WAKE: u32 = WM_APP + 1;

/// What the pet's own menu can ask the *app* to do.
///
/// The pet thread must not touch Tauri, so the window procedure only calls these
/// closures, which `lib.rs` hands in already bound to the main thread.
pub struct PetActions {
    pub settings: Box<dyn Fn() + Send + Sync>,
    pub quit: Box<dyn Fn() + Send + Sync>,
}

/// Set once at spawn: the window procedure is an `extern "system"` free function,
/// so it cannot close over the actions and reads them from here instead.
static ACTIONS: OnceLock<PetActions> = OnceLock::new();

const MENU_EMOTE: usize = 1;
const MENU_SETTINGS: usize = 2;
const MENU_QUIT: usize = 3;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// The pet's own right-click menu, at the cursor.
///
/// It must run on the thread that owns the window, so the pet holds its last
/// frame while the menu is open — a menu is modal. Settings and Quit go through
/// `ACTIONS`, because the pet thread cannot touch Tauri.
fn pet_menu(hwnd: HWND) {
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        AppendMenuW(menu, MF_STRING, MENU_EMOTE, wide("Emote now").as_ptr());
        AppendMenuW(menu, MF_STRING, MENU_SETTINGS, wide("Settings...").as_ptr());
        AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
        AppendMenuW(menu, MF_STRING, MENU_QUIT, wide("Quit").as_ptr());

        let mut p = POINT { x: 0, y: 0 };
        GetCursorPos(&mut p);
        // MSDN: the owner has to be foreground, or the menu will not dismiss when
        // you click somewhere else.
        SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            p.x,
            p.y,
            0,
            hwnd,
            ptr::null(),
        ) as usize;
        PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(menu);

        match cmd {
            MENU_EMOTE => EMOTE_REQUESTED.store(true, Ordering::SeqCst),
            MENU_SETTINGS => {
                if let Some(a) = ACTIONS.get() {
                    (a.settings)();
                }
            }
            MENU_QUIT => {
                if let Some(a) = ACTIONS.get() {
                    (a.quit)();
                }
            }
            _ => {}
        }
    }
}

/// The same resting animations the webview scheduler played.
const EMOTES: [StateId; 10] = [
    StateId::Wink,
    StateId::Wide,
    StateId::Notify,
    StateId::Exclaim,
    StateId::Play,
    StateId::Orbit,
    StateId::Burst,
    StateId::Sleep,
    StateId::Thinking,
    StateId::Alert,
];

/// What the settings window can change, already resolved from `config.json`.
#[derive(Clone, Debug)]
pub struct PetConfig {
    pub shape: ShapeId,
    pub expression: ExprId,
    pub color: String,
    pub style: String,
    /// Whether the face has a mouth; appearance, so it is applied in place with
    /// the rest of the config.
    pub mouth: bool,
    /// Whether the pet may wear the machine's mood instead of the expression you
    /// chose, while the box is busy or the battery is low.
    pub mood: bool,
    /// Idle minutes before it lies down; `0` means never.
    pub sleep_minutes: u32,
}

impl PetConfig {
    pub fn from_raw(
        shape: &str,
        expression: &str,
        color_id: &str,
        style: &str,
        mouth: bool,
        mood: bool,
        sleep_minutes: u32,
    ) -> Self {
        let hex = catalogue()
            .colors
            .iter()
            .find(|c| c.id == color_id)
            .map(|c| c.hex.clone())
            .unwrap_or_else(|| "#0a0a0c".into());
        Self {
            shape: ShapeId::from_id(shape).unwrap_or(ShapeId::Cercle),
            expression: ExprId::from_id(expression).unwrap_or(ExprId::Neutre),
            color: hex,
            style: style.to_string(),
            mouth,
            mood,
            sleep_minutes,
        }
    }
}

/// Shared handle: the settings window writes a new config, the pet thread reads
/// it every frame and applies it in place (no window reload, no flash).
#[derive(Clone)]
pub struct PetHandle {
    shared: Arc<Mutex<PetConfig>>,
    version: Arc<AtomicU64>,
    /// Whether the window should be on screen; the pet thread applies it.
    visible: Arc<AtomicBool>,
}

impl PetHandle {
    pub fn set(&self, config: PetConfig) {
        if let Ok(mut guard) = self.shared.lock() {
            *guard = config;
        }
        self.version.fetch_add(1, Ordering::Relaxed);
    }

    pub fn set_visible(&self, visible: bool) {
        self.visible.store(visible, Ordering::SeqCst);
    }

    pub fn is_visible(&self) -> bool {
        self.visible.load(Ordering::SeqCst)
    }
}

fn rand01() -> f64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0) as u64;
    let mut x = nanos
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    (x >> 11) as f64 / (1u64 << 53) as f64
}

/// A top-down 32-bit DIB the layered window is blitted from.
unsafe fn create_dib(memdc: HDC, size: i32) -> (HBITMAP, *mut c_void) {
    let mut bmi: BITMAPINFO = std::mem::zeroed();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = size;
    bmi.bmiHeader.biHeight = -size; // top-down
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;
    let mut bits: *mut c_void = ptr::null_mut();
    let hbitmap = CreateDIBSection(memdc, &bmi, DIB_RGB_COLORS, &mut bits, ptr::null_mut(), 0);
    (hbitmap, bits)
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_LBUTTONDOWN => {
            // A second click within the double-click window is a wink; the first
            // is a drag (the body acts as the title bar).
            let now = GetTickCount64();
            let last = LAST_CLICK.swap(now, Ordering::Relaxed);
            if now.saturating_sub(last) < DOUBLE_CLICK_MS {
                WINK_REQUESTED.store(true, Ordering::SeqCst);
                return 0;
            }
            let mut p = POINT { x: 0, y: 0 };
            GetCursorPos(&mut p);
            let lparam = ((p.y << 16) | (p.x & 0xffff)) as isize;
            // The OS runs the move loop inside this call, so the pet thread is
            // frozen while it is dragged. Compare the position before and after:
            // if it moved this was a drag, and the pet gets a startled look once
            // it is put down.
            let mut before: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut before);
            ReleaseCapture();
            SendMessageW(hwnd, WM_NCLBUTTONDOWN, HTCAPTION, lparam);
            let mut after: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut after);
            if before.left != after.left || before.top != after.top {
                WIDE_REQUESTED.store(true, Ordering::SeqCst);
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_RBUTTONUP => {
            pet_menu(hwnd);
            0
        }
        WM_APP_WAKE => {
            SHOW_REQUESTED.store(true, Ordering::SeqCst);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

/// The pet remembers where it was left, in physical pixels.
#[derive(Serialize, Deserialize)]
struct SavedPosition {
    x: i32,
    y: i32,
}

/// The last window position, or `None` if there is none / it is unreadable.
pub fn load_position(path: &Path) -> Option<(i32, i32)> {
    let text = std::fs::read_to_string(path).ok()?;
    let pos: SavedPosition = serde_json::from_str(&text).ok()?;
    Some((pos.x, pos.y))
}

fn save_position(path: &Path, x: i32, y: i32) {
    if let Ok(text) = serde_json::to_string(&SavedPosition { x, y }) {
        let _ = std::fs::write(path, text);
    }
}

/// Keep the window on a monitor's work area, so a remembered position can never
/// bring it back off-screen (monitor unplugged, resolution changed). The monitor
/// is chosen by the point itself, so a pet parked on a second screen stays there.
pub fn clamp_to_work_area(x: i32, y: i32, size: i32) -> (i32, i32) {
    unsafe {
        let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return (x, y);
        }
        let work = info.rcWork;
        let max_x = (work.right - size).max(work.left);
        let max_y = (work.bottom - size).max(work.top);
        (x.clamp(work.left, max_x), y.clamp(work.top, max_y))
    }
}

static REGISTER: Once = Once::new();

pub fn spawn(
    initial: PetConfig,
    actions: PetActions,
    x: i32,
    y: i32,
    size: i32,
    scale: f64,
    state_path: Option<PathBuf>,
) -> PetHandle {
    let _ = ACTIONS.set(actions);
    let shared = Arc::new(Mutex::new(initial));
    let version = Arc::new(AtomicU64::new(0));
    let visible = Arc::new(AtomicBool::new(true));
    let handle = PetHandle {
        shared: shared.clone(),
        version: version.clone(),
        visible: visible.clone(),
    };
    let stop = Arc::new(AtomicBool::new(false));
    thread::spawn(move || {
        run(
            shared, version, visible, stop, x, y, size, scale, state_path,
        )
    });
    handle
}

#[allow(clippy::too_many_arguments)]
fn run(
    shared: Arc<Mutex<PetConfig>>,
    version: Arc<AtomicU64>,
    visible: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    x: i32,
    y: i32,
    size: i32,
    scale: f64,
    state_path: Option<PathBuf>,
) {
    unsafe {
        let hinstance = GetModuleHandleW(ptr::null());
        let class = wide(PET_CLASS);
        REGISTER.call_once(|| {
            let wc = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: ptr::null_mut(),
                hCursor: ptr::null_mut(),
                hbrBackground: ptr::null_mut(),
                lpszMenuName: ptr::null(),
                lpszClassName: class.as_ptr(),
            };
            RegisterClassW(&wc);
        });

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
            class.as_ptr(),
            ptr::null(),
            WS_POPUP,
            x,
            y,
            size,
            size,
            ptr::null_mut(),
            ptr::null_mut(),
            hinstance,
            ptr::null(),
        );
        if hwnd.is_null() {
            return;
        }
        ShowWindow(hwnd, SW_SHOW);

        let memdc = CreateCompatibleDC(ptr::null_mut());
        let mut size = size;
        let (mut hbitmap, mut bits) = create_dib(memdc, size);
        SelectObject(memdc, hbitmap as HGDIOBJ);
        let mut buf = slice::from_raw_parts_mut(bits as *mut u8, (size * size * 4) as usize);

        let mut sz = SIZE { cx: size, cy: size };
        let src = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };

        let first = shared.lock().map(|c| c.clone()).unwrap_or(PetConfig {
            shape: ShapeId::Cercle,
            expression: ExprId::Neutre,
            color: "#0a0a0c".into(),
            style: "plat".into(),
            mouth: true,
            mood: true,
            sleep_minutes: 15,
        });

        let mut engine =
            BotEngine::new(R, StateId::Idle, Some(first.shape), Some(first.expression));
        engine.set_mouth(first.mouth);
        let mut color = first.color.clone();
        let mut style = first.style.clone();
        let mut pixmap = Pixmap::new(size as u32, size as u32).expect("pixmap");

        let start = Instant::now();
        let mut last_draw = 0u64;
        let mut wake_until = 0u64;
        let mut cursor_last = (i32::MIN, i32::MIN);
        let mut next_emote_at = 7.0 + rand01() * 6.0;
        let mut emote_ends_at = 0.0f64;
        let mut through = true;
        let mut frame = engine.sample(0.0);
        let mut s = size as f64 / (2.0 * DEMI);
        // The logical (DPI-independent) size the window should keep, and the DPI
        // it currently has.
        let logical = size as f64 / scale;
        let mut cur_dpi = GetDpiForWindow(hwnd) as f64;

        render::paint_into(&mut pixmap, &frame, &color, &style);
        render::to_bgra(&pixmap, buf);
        UpdateLayeredWindow(
            hwnd,
            ptr::null_mut(),
            ptr::null(),
            &sz,
            memdc,
            &src,
            0,
            &blend,
            ULW_ALPHA,
        );

        let mut last_version = version.load(Ordering::Relaxed);
        let mut shown = true;
        let mut saved_pos = (x, y);
        let mut sleeping = false;
        // Machine mood: the box's own state, and the expression the pet wears
        // because of it. `user_expr` is what the config asks for, `engine_expr`
        // what the engine currently has, so a change is applied exactly once.
        let mut load_sampler = LoadSampler::default();
        let mut power_at = 0u64;
        let mut power = (false, 100u8);
        let mut mood = Mood::Calm;
        let mut mood_on = first.mood;
        let mut sleep_minutes = first.sleep_minutes;
        let mut user_expr = first.expression;
        let mut engine_expr = first.expression;

        loop {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    DeleteObject(hbitmap as HGDIOBJ);
                    DeleteDC(memdc);
                    return;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if stop.load(Ordering::Relaxed) {
                break;
            }

            // Shown or hidden, set from the tray. While hidden the loop stays
            // cheap: no cursor poll, no paint.
            let want_visible = visible.load(Ordering::SeqCst);
            if want_visible != shown {
                shown = want_visible;
                ShowWindow(hwnd, if shown { SW_SHOW } else { SW_HIDE });
            }
            if !shown {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            // A monitor with a different scale factor changes this window's DPI.
            // Resize in place so the pet keeps the same physical size.
            let dpi = GetDpiForWindow(hwnd) as f64;
            if dpi > 0.0 && dpi != cur_dpi {
                cur_dpi = dpi;
                let want = (logical * dpi / 96.0).round() as i32;
                if want != size && want > 0 {
                    let mut r: RECT = std::mem::zeroed();
                    GetWindowRect(hwnd, &mut r);
                    size = want;
                    s = size as f64 / (2.0 * DEMI);
                    sz = SIZE { cx: size, cy: size };
                    pixmap = Pixmap::new(size as u32, size as u32).expect("pixmap");
                    DeleteObject(hbitmap as HGDIOBJ);
                    let (h, b) = create_dib(memdc, size);
                    hbitmap = h;
                    bits = b;
                    SelectObject(memdc, hbitmap as HGDIOBJ);
                    buf = slice::from_raw_parts_mut(bits as *mut u8, (size * size * 4) as usize);
                    SetWindowPos(
                        hwnd,
                        ptr::null_mut(),
                        r.left,
                        r.top,
                        size,
                        size,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
            }

            let now = start.elapsed().as_secs_f64();
            let now_ms = start.elapsed().as_millis() as u64;

            // Read the window rectangle once; the reactions, the gaze and the
            // position save all want it.
            let mut rect: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut rect);

            // A config change is rare; the version counter keeps the hot loop
            // from taking the lock on every wake-up.
            let v = version.load(Ordering::Relaxed);
            if v != last_version {
                last_version = v;
                if let Ok(cfg) = shared.lock().map(|c| c.clone()) {
                    engine.set_shape(Some(cfg.shape), now);
                    engine.set_mouth(cfg.mouth);
                    color = cfg.color;
                    style = cfg.style;
                    // The expression is not set here: the mood below decides it,
                    // and this is only what the config is asking for.
                    user_expr = cfg.expression;
                    mood_on = cfg.mood;
                    sleep_minutes = cfg.sleep_minutes;
                    // "Never" should mean awake, not awake eventually: a pet that
                    // was already asleep gets up as soon as the delay is turned off.
                    if sleep_minutes == 0 && sleeping {
                        sleeping = false;
                        engine.set_state(StateId::Idle, now);
                    }
                }
            }

            // The machine's mood, on its own slow clock. Only the resting face
            // carries it — an emote has a face of its own — so it is applied like
            // a config change, in place, and the expression you chose comes back
            // the moment the machine goes quiet.
            if mood_on {
                if now_ms.saturating_sub(power_at) >= POWER_SAMPLE_MS {
                    power_at = now_ms;
                    power = power_status();
                }
                mood = mood_for(load_sampler.load(now_ms), power.0, power.1, mood);
            } else {
                mood = Mood::Calm;
            }
            let wanted = mood_expression(mood, user_expr);
            if wanted != engine_expr {
                engine.set_expression(Some(wanted), now);
                engine_expr = wanted;
            }

            let mut pos = POINT { x: 0, y: 0 };
            GetCursorPos(&mut pos);
            if pos.x != cursor_last.0 || pos.y != cursor_last.1 {
                cursor_last = (pos.x, pos.y);
                wake_until = now_ms + WAKE_MS;
            }

            // Reactions. A double-click winks, a drag that put the pet down gets a
            // startled look, and a spell with no input anywhere sends it to sleep —
            // all on states the engine already has.
            let in_window = pos.x >= rect.left
                && pos.x < rect.right
                && pos.y >= rect.top
                && pos.y < rect.bottom;

            if WINK_REQUESTED.swap(false, Ordering::SeqCst) {
                sleeping = false;
                engine.set_state(StateId::Wink, now);
                let duration = catalogue().state_meta(StateId::Wink).duration.max(1.6);
                emote_ends_at = now + duration;
                next_emote_at = emote_ends_at + 7.0 + rand01() * 14.0;
            }
            if WIDE_REQUESTED.swap(false, Ordering::SeqCst) {
                sleeping = false;
                engine.set_state(StateId::Wide, now);
                let duration = catalogue().state_meta(StateId::Wide).duration.max(1.6);
                emote_ends_at = now + duration;
                next_emote_at = emote_ends_at + 7.0 + rand01() * 14.0;
            }
            if SHOW_REQUESTED.swap(false, Ordering::SeqCst) {
                // A second Dot was launched and asked this one to come back on
                // screen; show it through the same flag the tray writes, so the
                // two cannot disagree.
                visible.store(true, Ordering::SeqCst);
                SetForegroundWindow(hwnd);
            }
            if EMOTE_REQUESTED.swap(false, Ordering::SeqCst) {
                // "Emote now" from the pet's own menu: the same path a spontaneous
                // emote takes, just asked for.
                sleeping = false;
                let id = EMOTES[(rand01() * EMOTES.len() as f64) as usize % EMOTES.len()];
                engine.set_state(id, now);
                let duration = catalogue().state_meta(id).duration.max(1.6);
                emote_ends_at = now + duration;
                next_emote_at = emote_ends_at + 7.0 + rand01() * 14.0;
            }

            if sleeping {
                // Anything in the window — the cursor passing over it, a click —
                // wakes the pet, and waking is a yawn: the one state that shows the
                // mouth at its widest. It runs as an emote, so the state machine
                // already knows how to put the pet back to Idle afterwards.
                if in_window {
                    sleeping = false;
                    engine.set_state(StateId::Yawn, now);
                    let duration = catalogue().state_meta(StateId::Yawn).duration.max(1.8);
                    emote_ends_at = now + duration;
                    next_emote_at = emote_ends_at + 7.0 + rand01() * 14.0;
                }
            } else if should_sleep(
                system_idle_ms(),
                sleep_after_ms(sleep_minutes),
                in_window,
                emote_ends_at > 0.0,
            ) {
                engine.set_state(StateId::Sleep, now);
                sleeping = true;
            }

            if !sleeping {
                if emote_ends_at > 0.0 {
                    if now >= emote_ends_at {
                        emote_ends_at = 0.0;
                        engine.set_state(StateId::Idle, now);
                    }
                } else if now >= next_emote_at {
                    let id = EMOTES[(rand01() * EMOTES.len() as f64) as usize % EMOTES.len()];
                    engine.set_state(id, now);
                    let duration = catalogue().state_meta(id).duration.max(1.6);
                    emote_ends_at = now + duration;
                    next_emote_at = emote_ends_at + 7.0 + rand01() * 14.0;
                }
            }

            let busy = emote_ends_at > 0.0 || now_ms < wake_until;
            let interval = if busy { BUSY_MS } else { IDLE_MS };

            if now_ms.saturating_sub(last_draw) >= interval {
                last_draw = now_ms;

                // A drag moves the window while this thread is blocked in the OS
                // move loop; the first frame after release sees the new position.
                if let Some(path) = &state_path {
                    if (rect.left, rect.top) != saved_pos {
                        save_position(path, rect.left, rect.top);
                        saved_pos = (rect.left, rect.top);
                    }
                }
                // Gaze is normalised against the monitor the pet is on, not the
                // primary one, so a pet parked on a second screen still looks at
                // the cursor correctly.
                let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                let mut minfo: MONITORINFO = std::mem::zeroed();
                minfo.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
                let (mon_w, mon_h) = if GetMonitorInfoW(monitor, &mut minfo) != 0 {
                    (
                        (minfo.rcMonitor.right - minfo.rcMonitor.left) as f64,
                        (minfo.rcMonitor.bottom - minfo.rcMonitor.top) as f64,
                    )
                } else {
                    (1920.0, 1080.0)
                };
                let cx = (rect.left + rect.right) as f64 / 2.0;
                let cy = (rect.top + rect.bottom) as f64 / 2.0;
                let nx = ((pos.x as f64 - cx) / (mon_w / 2.0).max(1.0)).clamp(-1.0, 1.0);
                let ny = ((pos.y as f64 - cy) / (mon_h / 2.0).max(1.0)).clamp(-1.0, 1.0);
                engine.set_look(
                    Some(Look {
                        yaw: nx * YAW_MAX,
                        pitch: PITCH_BASE - ny * PITCH_MAX,
                        mix: 1.0,
                        spin: 0.0,
                        wander: 0.0,
                    }),
                    now,
                    BotEngine::LOOK_MORPH,
                );

                frame = engine.sample(now);

                // Hit test in viewBox units, with the same five-point hysteresis
                // the webview used, then flip click-through.
                let lx = (pos.x as f64 - rect.left as f64) / s - DEMI;
                let ly = (pos.y as f64 - rect.top as f64) / s - DEMI;
                let pad = 5.0 / s;
                let probes = [(0.0, 0.0), (pad, 0.0), (-pad, 0.0), (0.0, pad), (0.0, -pad)];
                let over = probes
                    .iter()
                    .any(|(dx, dy)| render::body_contains(&frame.body_points, lx + dx, ly + dy));
                let want_through = !over;
                if want_through != through {
                    through = want_through;
                    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                    let flag = WS_EX_TRANSPARENT as isize;
                    let next = if through { ex | flag } else { ex & !flag };
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next);
                }

                render::paint_into(&mut pixmap, &frame, &color, &style);
                render::to_bgra(&pixmap, buf);
                // pptDst NULL: keep the position the OS drag gave the window.
                UpdateLayeredWindow(
                    hwnd,
                    ptr::null_mut(),
                    ptr::null(),
                    &sz,
                    memdc,
                    &src,
                    0,
                    &blend,
                    ULW_ALPHA,
                );
            }

            // Wake at about the draw cadence: 60 Hz-ish while busy, 15 Hz-ish
            // while resting. A fixed 4 ms sleep was most of the idle CPU.
            let sleep_ms = if busy { 8 } else { 20 };
            thread::sleep(Duration::from_millis(sleep_ms));
        }

        DeleteObject(hbitmap as HGDIOBJ);
        DeleteDC(memdc);
        DestroyWindow(hwnd);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        busy_fraction, mood_expression, mood_for, should_sleep, sleep_after_ms, ExprId, Mood,
    };

    #[test]
    fn sleep_after_reads_minutes_and_zero_means_never() {
        assert_eq!(sleep_after_ms(5), Some(5 * 60_000));
        assert_eq!(sleep_after_ms(15), Some(15 * 60_000));
        assert_eq!(sleep_after_ms(0), None);
        // "Never" means never, however long the machine has been quiet.
        assert!(!should_sleep(u64::MAX, None, false, false));
    }

    #[test]
    fn sleeps_only_after_the_whole_session_goes_quiet() {
        let after = sleep_after_ms(15);
        // Working anywhere on the PC keeps it awake, even with the cursor nowhere
        // near the pet.
        assert!(!should_sleep(15 * 60_000 - 1, after, false, false));
        // A quarter of an hour with no input anywhere: it lies down.
        assert!(should_sleep(15 * 60_000, after, false, false));
        assert!(should_sleep(15 * 60_000 * 10, after, false, false));
    }

    #[test]
    fn stays_awake_under_a_parked_pointer_or_a_running_emote() {
        let after = sleep_after_ms(15);
        // A pointer parked on the pet reads as an idle session, but the pet must
        // not sleep there — it would wake itself with a yawn and repeat.
        assert!(!should_sleep(15 * 60_000 * 10, after, true, false));
        // An emote in flight is not a moment to lie down.
        assert!(!should_sleep(15 * 60_000 * 10, after, false, true));
    }

    #[test]
    fn busy_fraction_ignores_the_idle_time() {
        // `kernel` already contains `idle`: 100 ticks passed, 90 of them idle.
        let load = busy_fraction((0, 0, 0), (90, 60, 40)).unwrap();
        assert!((load - 0.10).abs() < 1e-9, "got {load}");
        // No ticks elapsed: no reading, rather than a division by zero.
        assert!(busy_fraction((90, 60, 40), (90, 60, 40)).is_none());
    }

    #[test]
    fn a_quiet_machine_is_calm_and_a_pegged_one_is_frazzled() {
        assert_eq!(mood_for(0.05, false, 100, Mood::Calm), Mood::Calm);
        assert_eq!(mood_for(0.60, false, 100, Mood::Calm), Mood::Busy);
        assert_eq!(mood_for(0.95, false, 100, Mood::Calm), Mood::Pegged);
    }

    #[test]
    fn the_machine_has_to_fall_well_below_the_line_to_calm_down() {
        // Hysteresis: a busy machine holds its mood down to 0.30 and a pegged one
        // down to 0.70, so a load sitting on a line does not flicker the face.
        assert_eq!(mood_for(0.40, false, 100, Mood::Busy), Mood::Busy);
        assert_eq!(mood_for(0.20, false, 100, Mood::Busy), Mood::Calm);
        assert_eq!(mood_for(0.75, false, 100, Mood::Pegged), Mood::Pegged);
        assert_eq!(mood_for(0.60, false, 100, Mood::Pegged), Mood::Busy);
    }

    #[test]
    fn a_low_battery_outranks_the_load_and_clears_when_plugged_in() {
        assert_eq!(mood_for(0.95, true, 10, Mood::Calm), Mood::LowBattery);
        // Plugged in at the same level: the load decides again.
        assert_eq!(mood_for(0.05, false, 10, Mood::Calm), Mood::Calm);
        // A desktop on mains reports no battery at all, and must stay calm.
        assert_eq!(mood_for(0.05, false, 100, Mood::Calm), Mood::Calm);
    }

    #[test]
    fn a_calm_machine_wears_the_expression_you_chose() {
        assert_eq!(mood_expression(Mood::Calm, ExprId::Hilare), ExprId::Hilare);
        assert_eq!(
            mood_expression(Mood::Busy, ExprId::Hilare),
            ExprId::Attentif
        );
        assert_eq!(
            mood_expression(Mood::Pegged, ExprId::Hilare),
            ExprId::Effraye
        );
        assert_eq!(
            mood_expression(Mood::LowBattery, ExprId::Hilare),
            ExprId::Somnolent
        );
    }
}
