/// Platform-specific setup for standalone window mode.
///
/// On Windows release builds: `windows_subsystem = "windows"` suppresses
/// the default console. We relaunch through `conhost.exe` to get a dedicated
/// console window with our custom icon (bypasses Windows Terminal, which would
/// override our icon and title bar). Falls back to `AllocConsole` if that fails.
///
/// On Windows debug builds: reuses the existing terminal, sets title and icon.
/// On other platforms: no-op.

#[cfg(windows)]
mod windows_setup {
    use std::ptr;

    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::System::Console::{
        AllocConsole, GetConsoleWindow, SetConsoleTitleW,
        SetConsoleScreenBufferSize, SetConsoleWindowInfo,
        GetStdHandle, STD_OUTPUT_HANDLE,
        COORD, SMALL_RECT,
        CONSOLE_FONT_INFOEX, SetCurrentConsoleFontEx,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SetWindowPos, SendMessageW, LoadImageW,
        SetForegroundWindow,
        SM_CXSCREEN, SM_CYSCREEN,
        SWP_NOZORDER, SWP_NOACTIVATE, SWP_NOSIZE,
        WM_SETICON, ICON_BIG, ICON_SMALL,
        IMAGE_ICON,
        GetWindowRect,
    };

    fn is_invalid_handle(h: *mut core::ffi::c_void) -> bool {
        h.is_null() || h == -1_isize as *mut _
    }

    pub fn setup() {
        let existing = unsafe { GetConsoleWindow() };

        if existing.is_null() {
            // No console — release mode with windows_subsystem = "windows".
            // Relaunch through conhost.exe for a standalone window with our icon.
            // Without this, Win11 routes AllocConsole through Windows Terminal,
            // which ignores our WM_SETICON calls.
            if std::env::var("_WRITE_CONHOST").is_err() {
                if relaunch_via_conhost() {
                    std::process::exit(0);
                }
            }
            // Relaunched or relaunch failed — ensure we have a console
            if unsafe { GetConsoleWindow() }.is_null() {
                unsafe {
                    AllocConsole();
                }
            }
        }

        set_title();
        set_icon();
        configure_console_font();
        set_window_size(120, 35);
        center_window();

        // Ensure window is brought to front
        let hwnd = unsafe { GetConsoleWindow() };
        if !hwnd.is_null() {
            unsafe {
                SetForegroundWindow(hwnd);
            }
        }
    }

    /// Relaunch through conhost.exe to bypass Windows Terminal and get
    /// a dedicated console window where we can set our own icon.
    fn relaunch_via_conhost() -> bool {
        let exe = match std::env::current_exe() {
            Ok(p) => p,
            Err(_) => return false,
        };

        let args: Vec<String> = std::env::args().skip(1).collect();

        std::process::Command::new("conhost.exe")
            .arg("--")
            .arg(&exe)
            .args(&args)
            .env("_WRITE_CONHOST", "1")
            .spawn()
            .is_ok()
    }

    fn set_title() {
        let title: Vec<u16> = "Write\0".encode_utf16().collect();
        unsafe {
            SetConsoleTitleW(title.as_ptr());
        }
    }

    fn set_icon() {
        let hwnd = unsafe { GetConsoleWindow() };
        if hwnd.is_null() {
            return;
        }

        let hmodule = unsafe { GetModuleHandleW(ptr::null()) };
        if hmodule.is_null() {
            return;
        }

        // MAKEINTRESOURCE(1) — icon resource ID set by winresource in build.rs
        let res_id = 1usize as *const u16;

        // Load big icon (for title bar and alt-tab)
        let hicon_big = unsafe { LoadImageW(hmodule, res_id, IMAGE_ICON, 32, 32, 0) };
        // Load small icon (for window corner and taskbar)
        let hicon_small = unsafe { LoadImageW(hmodule, res_id, IMAGE_ICON, 16, 16, 0) };

        if !hicon_big.is_null() {
            unsafe {
                SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, hicon_big as isize);
            }
        }
        if !hicon_small.is_null() {
            unsafe {
                SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, hicon_small as isize);
            }
        }
    }

    fn configure_console_font() {
        let handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
        if is_invalid_handle(handle) {
            return;
        }

        let font_name = "Cascadia Mono";
        let mut face_name = [0u16; 32];
        for (i, c) in font_name.encode_utf16().enumerate() {
            if i >= 31 {
                break;
            }
            face_name[i] = c;
        }

        let mut font_info = CONSOLE_FONT_INFOEX {
            cbSize: std::mem::size_of::<CONSOLE_FONT_INFOEX>() as u32,
            nFont: 0,
            dwFontSize: COORD { X: 0, Y: 20 },
            FontFamily: 54, // TrueType + fixed pitch
            FontWeight: 400,
            FaceName: face_name,
        };

        unsafe {
            SetCurrentConsoleFontEx(handle, 0, &mut font_info);
        }
    }

    fn set_window_size(cols: i16, rows: i16) {
        let handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
        if is_invalid_handle(handle) {
            return;
        }

        // Shrink window to minimum first to avoid buffer < window errors
        let small_rect = SMALL_RECT {
            Left: 0,
            Top: 0,
            Right: 1,
            Bottom: 1,
        };
        unsafe {
            SetConsoleWindowInfo(handle, 1, &small_rect);
        }

        // Set buffer size
        let buffer_size = COORD { X: cols, Y: 9999 };
        unsafe {
            SetConsoleScreenBufferSize(handle, buffer_size);
        }

        // Set window size (right/bottom are inclusive)
        let window_rect = SMALL_RECT {
            Left: 0,
            Top: 0,
            Right: cols - 1,
            Bottom: rows - 1,
        };
        unsafe {
            SetConsoleWindowInfo(handle, 1, &window_rect);
        }
    }

    fn center_window() {
        let hwnd = unsafe { GetConsoleWindow() };
        if hwnd.is_null() {
            return;
        }

        let (screen_w, screen_h) = unsafe {
            (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN))
        };

        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        unsafe {
            GetWindowRect(hwnd, &mut rect);
        }

        let win_w = rect.right - rect.left;
        let win_h = rect.bottom - rect.top;
        let x = (screen_w - win_w) / 2;
        let y = (screen_h - win_h) / 2;

        unsafe {
            SetWindowPos(
                hwnd,
                ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOSIZE,
            );
        }
    }
}

#[cfg(windows)]
pub fn setup() {
    windows_setup::setup();
}

#[cfg(not(windows))]
pub fn setup() {
    // No-op on non-Windows platforms
}
