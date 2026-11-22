//! Native Win32 helpers. These replace PowerShell round-trips, which add
//! ~1s of latency each and can flash console windows.

#[cfg(windows)]
mod ffi {
    use std::ffi::c_void;
    pub type Hwnd = *mut c_void;

    #[link(name = "user32")]
    extern "system" {
        pub fn GetForegroundWindow() -> Hwnd;
        pub fn GetWindow(hwnd: Hwnd, command: u32) -> Hwnd;
        pub fn IsWindowVisible(hwnd: Hwnd) -> i32;
        pub fn GetClassNameW(hwnd: Hwnd, class_name: *mut u16, max_count: i32) -> i32;
        pub fn GetWindowTextW(hwnd: Hwnd, text: *mut u16, max_count: i32) -> i32;
        pub fn ShowWindow(hwnd: Hwnd, cmd: i32) -> i32;
        pub fn PostMessageW(hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> i32;
        pub fn keybd_event(vk: u8, scan: u8, flags: u32, extra: usize);
    }

    #[link(name = "shell32")]
    extern "system" {
        pub fn ShellExecuteW(
            hwnd: Hwnd,
            operation: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show_cmd: i32,
        ) -> isize;
    }

    pub const GW_HWNDNEXT: u32 = 2;
    pub const SW_SHOWNORMAL: i32 = 1;
    pub const SW_MAXIMIZE: i32 = 3;
    pub const SW_MINIMIZE: i32 = 6;
    pub const SW_RESTORE: i32 = 9;
    pub const WM_CLOSE: u32 = 0x0010;
    pub const KEYEVENTF_KEYUP: u32 = 0x0002;

    pub unsafe fn title(hwnd: Hwnd) -> String {
        let mut buf = [0u16; 256];
        let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        String::from_utf16_lossy(&buf[..len.max(0) as usize])
    }

    pub unsafe fn class(hwnd: Hwnd) -> String {
        let mut buf = [0u16; 256];
        let len = GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        String::from_utf16_lossy(&buf[..len.max(0) as usize])
    }

    pub fn is_shell_class(class: &str) -> bool {
        matches!(class, "Progman" | "WorkerW" | "Shell_TrayWnd")
    }

    pub fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

/// The window the user is working in. Byte is topmost and often foreground
/// after a click, so skip past it to the next real window in Z order.
#[cfg(windows)]
fn user_window() -> Option<ffi::Hwnd> {
    unsafe {
        let mut hwnd = ffi::GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        if ffi::title(hwnd).eq_ignore_ascii_case("Byte") {
            hwnd = ffi::GetWindow(hwnd, ffi::GW_HWNDNEXT);
            while !hwnd.is_null() {
                let visible = ffi::IsWindowVisible(hwnd) != 0;
                if visible && (!ffi::title(hwnd).is_empty() || ffi::is_shell_class(&ffi::class(hwnd))) {
                    break;
                }
                hwnd = ffi::GetWindow(hwnd, ffi::GW_HWNDNEXT);
            }
        }
        (!hwnd.is_null()).then_some(hwnd)
    }
}

/// Title of the window the user is working in, if it is an app window.
pub fn get_active_window_title() -> String {
    #[cfg(windows)]
    unsafe {
        if let Some(hwnd) = user_window() {
            if !ffi::is_shell_class(&ffi::class(hwnd)) {
                return ffi::title(hwnd).trim().to_string();
            }
        }
    }
    String::new()
}

/// Returns true when the Windows desktop shell, rather than an app window, is foreground.
pub fn foreground_is_desktop() -> Option<bool> {
    #[cfg(windows)]
    unsafe {
        if ffi::GetForegroundWindow().is_null() {
            return Some(true);
        }
        let hwnd = user_window()?;
        Some(ffi::is_shell_class(&ffi::class(hwnd)))
    }
    #[cfg(not(windows))]
    {
        Some(false)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum WindowAction {
    Minimize,
    Maximize,
    Restore,
    Close,
}

/// Apply an action to the user's current app window. Returns the window title.
pub fn control_user_window(action: WindowAction) -> Result<String, String> {
    #[cfg(windows)]
    unsafe {
        let hwnd = user_window().ok_or("There's no app window to control.")?;
        if ffi::is_shell_class(&ffi::class(hwnd)) {
            return Err("There's no app window to control.".to_string());
        }
        let title = ffi::title(hwnd);
        match action {
            WindowAction::Minimize => { ffi::ShowWindow(hwnd, ffi::SW_MINIMIZE); }
            WindowAction::Maximize => { ffi::ShowWindow(hwnd, ffi::SW_MAXIMIZE); }
            WindowAction::Restore => { ffi::ShowWindow(hwnd, ffi::SW_RESTORE); }
            WindowAction::Close => { ffi::PostMessageW(hwnd, ffi::WM_CLOSE, 0, 0); }
        }
        Ok(title)
    }
    #[cfg(not(windows))]
    {
        let _ = action;
        Err("Window control is only supported on Windows.".to_string())
    }
}

/// Virtual-key codes for media and volume keys.
pub mod vk {
    pub const VOLUME_MUTE: u8 = 0xAD;
    pub const VOLUME_DOWN: u8 = 0xAE;
    pub const VOLUME_UP: u8 = 0xAF;
    pub const MEDIA_NEXT: u8 = 0xB0;
    pub const MEDIA_PREV: u8 = 0xB1;
    pub const MEDIA_PLAY_PAUSE: u8 = 0xB3;
}

/// Simulate pressing a key `times` times.
pub fn press_key(vk: u8, times: u32) {
    #[cfg(windows)]
    unsafe {
        for _ in 0..times.max(1) {
            ffi::keybd_event(vk, 0, 0, 0);
            ffi::keybd_event(vk, 0, ffi::KEYEVENTF_KEYUP, 0);
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (vk, times);
    }
}

/// Open a file, folder, URL, or program through the Windows shell, without
/// going through cmd.exe (so no shell metacharacter interpretation).
pub fn shell_open(target: &str, parameters: Option<&str>) -> Result<(), String> {
    #[cfg(windows)]
    unsafe {
        let op = ffi::wide("open");
        let file = ffi::wide(target);
        let params = parameters.map(ffi::wide);
        let result = ffi::ShellExecuteW(
            std::ptr::null_mut(),
            op.as_ptr(),
            file.as_ptr(),
            params.as_ref().map_or(std::ptr::null(), |p| p.as_ptr()),
            std::ptr::null(),
            ffi::SW_SHOWNORMAL,
        );
        if result > 32 {
            Ok(())
        } else {
            Err(format!("Windows couldn't open '{}' (code {}).", target, result))
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (target, parameters);
        Err("Opening items is only supported on Windows.".to_string())
    }
}
