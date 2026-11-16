//! One Dot per desktop.
//!
//! Nothing stops a second launch from creating a second pet — the window is
//! native and unsigned — so a named mutex holds the claim. A second process that
//! loses the race asks the Dot already running to show its pet, then steps aside
//! before it builds a tray or a window.

use std::sync::atomic::{AtomicIsize, Ordering};

use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW};

/// Held for the life of the process: dropping the handle would release the claim.
/// Windows owns the actual mutex, so this only keeps the handle from closing.
static CLAIM: AtomicIsize = AtomicIsize::new(0);

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Take the single-instance claim. True when another Dot already holds it.
///
/// `Local\` scopes the mutex to this login session, so two users on the same PC
/// each get their own pet.
pub fn already_running() -> bool {
    let name = wide("Local\\dev.dotscode.dot");
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        // We could not claim anything; run rather than refuse to start.
        return false;
    }
    CLAIM.store(handle as isize, Ordering::SeqCst);
    // `GetLastError` reports the existing mutex even though the call succeeded
    // and handed back a valid handle.
    unsafe { GetLastError() == ERROR_ALREADY_EXISTS }
}

/// Ask the Dot that is already running to show its pet again.
///
/// Returns false when no pet window was found, so the caller can say so rather
/// than vanish silently.
pub fn wake_running_pet() -> bool {
    let class = wide(crate::pet::PET_CLASS);
    let hwnd = unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) };
    if hwnd.is_null() {
        return false;
    }
    unsafe { PostMessageW(hwnd, crate::pet::WM_APP_WAKE, 0, 0) != 0 }
}
