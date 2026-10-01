//! Application-specific modules and workarounds.

pub mod chromium;
pub mod explorer;
pub mod firefox;

pub use chromium::ChromiumFilter;
pub use explorer::ExplorerFilter;
pub use firefox::FirefoxFilter;

/// Queries the executable image filename (e.g. "msedge.exe", "chrome.exe") for a process ID.
pub fn get_process_name_by_pid(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    unsafe {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
            PROCESS_QUERY_LIMITED_INFORMATION,
        };

        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 512];
        let mut size = buf.len() as u32;
        let res = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);

        if res.is_ok() && size > 0 {
            let full_path = String::from_utf16_lossy(&buf[..size as usize]);
            if let Some(filename) = full_path.rsplit(&['\\', '/'][..]).next() {
                return Some(filename.to_string());
            }
            return Some(full_path);
        }
        None
    }
}

