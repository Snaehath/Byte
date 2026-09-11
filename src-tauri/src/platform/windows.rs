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
