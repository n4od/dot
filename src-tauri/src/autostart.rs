//! Start Dot with Windows, via `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
//!
//! Windows-only by design, like the rest of the app. Keeping this here rather
//! than adding the cross-platform autostart plugin avoids a second runtime and an
//! npm package for one registry value.

use std::ptr;

use windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ,
};

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const VALUE_NAME: &str = "Dot";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// The command Windows should run: the current executable, quoted.
fn command() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(format!("\"{}\"", exe.display()))
}

fn open(access: u32) -> Result<HKEY, String> {
    let path = wide(RUN_KEY);
    let mut key: HKEY = ptr::null_mut();
    let rc = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, access, &mut key) };
    if rc != 0 {
        return Err(format!("RegOpenKeyExW failed ({rc})"));
    }
    Ok(key)
}

/// Whether a Run entry exists *and* still points at this executable. A stale
/// entry (the portable exe was moved) reads as off, so enabling rewrites it.
pub fn is_enabled() -> Result<bool, String> {
    let expected = command()?.to_lowercase();
    let key = open(KEY_QUERY_VALUE)?;
    let name = wide(VALUE_NAME);
    let mut buf = [0u16; 1024];
    let mut size = (buf.len() * 2) as u32;
    let mut kind = 0u32;
    let rc = unsafe {
        RegQueryValueExW(
            key,
            name.as_ptr(),
            ptr::null(),
            &mut kind,
            buf.as_mut_ptr() as *mut u8,
            &mut size,
        )
    };
    unsafe { RegCloseKey(key) };
    if rc != 0 || kind != REG_SZ {
        return Ok(false);
    }
    let len = ((size as usize) / 2).min(buf.len());
    let end = len.saturating_sub(1);
    let value = String::from_utf16_lossy(&buf[..end]);
    Ok(value.to_lowercase() == expected)
}

/// Enable or disable the Run entry.
pub fn set(enabled: bool) -> Result<(), String> {
    let key = open(KEY_SET_VALUE)?;
    let name = wide(VALUE_NAME);
    let rc = if enabled {
        let data = wide(&command()?);
        let bytes = (data.len() * 2) as u32;
        unsafe {
            RegSetValueExW(
                key,
                name.as_ptr(),
                0,
                REG_SZ,
                data.as_ptr() as *const u8,
                bytes,
            )
        }
    } else {
        unsafe { RegDeleteValueW(key, name.as_ptr()) }
    };
    unsafe { RegCloseKey(key) };
    // Deleting a value that is not there is success, not an error.
    if rc != 0 && (enabled || rc != ERROR_FILE_NOT_FOUND) {
        return Err(format!("registry update failed ({rc})"));
    }
    Ok(())
}
