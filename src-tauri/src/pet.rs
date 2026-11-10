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
use std::sync::{Arc, Mutex, Once};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tiny_skia::Pixmap;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetMonitorInfoW,
    MonitorFromPoint, MonitorFromWindow, SelectObject, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ, MONITORINFO,
    MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::SystemInformation::GetTickCount64;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos,
    GetWindowLongPtrW, GetWindowRect, PeekMessageW, PostQuitMessage, RegisterClassW, SendMessageW,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage, UpdateLayeredWindow,
    GWL_EXSTYLE, MSG, PM_REMOVE, SWP_NOACTIVATE, SWP_NOZORDER, SW_HIDE, SW_SHOW, ULW_ALPHA,
    WM_CLOSE, WM_DESTROY, WM_LBUTTONDOWN, WM_NCLBUTTONDOWN, WM_QUIT, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
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

/// Tick of the previous click, and a wink requested by a double-click. The engine
/// lives on the pet thread, so the window procedure only raises the flag.
static LAST_CLICK: AtomicU64 = AtomicU64::new(0);
static WINK_REQUESTED: AtomicBool = AtomicBool::new(false);

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
}

impl PetConfig {
    pub fn from_raw(shape: &str, expression: &str, color_id: &str, style: &str) -> Self {
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
            ReleaseCapture();
            SendMessageW(hwnd, WM_NCLBUTTONDOWN, HTCAPTION, lparam);
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
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
    x: i32,
    y: i32,
    size: i32,
    scale: f64,
    state_path: Option<PathBuf>,
) -> PetHandle {
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
        let class: Vec<u16> = "DotNativePet\0".encode_utf16().collect();
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
        });

        let mut engine =
            BotEngine::new(R, StateId::Idle, Some(first.shape), Some(first.expression));
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

            // A config change is rare; the version counter keeps the hot loop
            // from taking the lock on every wake-up.
            let v = version.load(Ordering::Relaxed);
            if v != last_version {
                last_version = v;
                if let Ok(cfg) = shared.lock().map(|c| c.clone()) {
                    engine.set_shape(Some(cfg.shape), now);
                    engine.set_expression(Some(cfg.expression), now);
                    color = cfg.color;
                    style = cfg.style;
                }
            }

            let mut pos = POINT { x: 0, y: 0 };
            GetCursorPos(&mut pos);
            if pos.x != cursor_last.0 || pos.y != cursor_last.1 {
                cursor_last = (pos.x, pos.y);
                wake_until = now_ms + WAKE_MS;
            }

            // A double-click asks for a wink; it takes priority over the schedule.
            if WINK_REQUESTED.swap(false, Ordering::SeqCst) {
                engine.set_state(StateId::Wink, now);
                let duration = catalogue().state_meta(StateId::Wink).duration.max(1.6);
                emote_ends_at = now + duration;
                next_emote_at = emote_ends_at + 7.0 + rand01() * 14.0;
            }

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

            let busy = emote_ends_at > 0.0 || now_ms < wake_until;
            let interval = if busy { BUSY_MS } else { IDLE_MS };

            if now_ms.saturating_sub(last_draw) >= interval {
                last_draw = now_ms;

                let mut rect: RECT = std::mem::zeroed();
                GetWindowRect(hwnd, &mut rect);
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
