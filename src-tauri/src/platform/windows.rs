pub fn get_active_window_title() -> String {
    let script = r#"
        $code = @'
        [DllImport("user32.dll")]
        public static extern IntPtr GetForegroundWindow();
        [DllImport("user32.dll", CharSet = CharSet.Auto)]
        public static extern int GetWindowText(IntPtr hWnd, System.Text.StringBuilder text, int count);
        '@
        $type = Add-Type -MemberDefinition $code -Name "Win32ActiveWindow" -PassThru -ErrorAction SilentlyContinue
        $hwnd = $type::GetForegroundWindow()
        $title = New-Object System.Text.StringBuilder 256
        $null = $type::GetWindowText($hwnd, $title, 256)
        $title.ToString()
    "#;

    let output = crate::Command::new("powershell")
        .args(&["-Command", script])
        .output();

    if let Ok(out) = output {
        let title_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if title_str.is_empty() {
            "Desktop / Windows".to_string()
        } else {
            title_str
        }
    } else {
        "Desktop / Windows".to_string()
    }
}

/// Returns true when the Windows desktop shell, rather than an app window, is foreground.
pub fn foreground_is_desktop() -> Option<bool> {
    #[cfg(windows)]
    unsafe {
        use std::ffi::c_void;

        #[link(name = "user32")]
        extern "system" {
            fn GetForegroundWindow() -> *mut c_void;
            fn GetWindow(hwnd: *mut c_void, command: u32) -> *mut c_void;
            fn IsWindowVisible(hwnd: *mut c_void) -> i32;
            fn GetClassNameW(hwnd: *mut c_void, class_name: *mut u16, max_count: i32) -> i32;
            fn GetWindowTextW(hwnd: *mut c_void, text: *mut u16, max_count: i32) -> i32;
        }

        let mut hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return Some(true);
        }

        // Byte is topmost and may become foreground after a click. Inspect the
        // next window in Z order so we still classify the app behind the orb.
        if window_title(hwnd, GetWindowTextW).eq_ignore_ascii_case("Byte") {
            let mut next_window = GetWindow(hwnd, 2); // GW_HWNDNEXT
            while !next_window.is_null() && IsWindowVisible(next_window) == 0 {
                next_window = GetWindow(next_window, 2);
            }
            hwnd = next_window;
            if hwnd.is_null() {
                return None;
            }
        }

        let class_name = window_class(hwnd, GetClassNameW);
        Some(matches!(class_name.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd"))
    }

    #[cfg(not(windows))]
    {
        Some(false)
    }
}

#[cfg(windows)]
unsafe fn window_title(
    hwnd: *mut std::ffi::c_void,
    get_window_text: unsafe extern "system" fn(*mut std::ffi::c_void, *mut u16, i32) -> i32,
) -> String {
    let mut title = [0u16; 256];
    let len = get_window_text(hwnd, title.as_mut_ptr(), title.len() as i32);
    String::from_utf16_lossy(&title[..len.max(0) as usize])
}

#[cfg(windows)]
unsafe fn window_class(
    hwnd: *mut std::ffi::c_void,
    get_class_name: unsafe extern "system" fn(*mut std::ffi::c_void, *mut u16, i32) -> i32,
) -> String {
    let mut class_name = [0u16; 256];
    let len = get_class_name(hwnd, class_name.as_mut_ptr(), class_name.len() as i32);
    String::from_utf16_lossy(&class_name[..len.max(0) as usize])
}
