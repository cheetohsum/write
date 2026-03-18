/// Platform-specific setup for standalone window mode.
///
/// On Windows: allocates a console (release) or reuses existing (debug),
/// sets the window title, icon, size, and centers it on screen.
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
        SM_CXSCREEN, SM_CYSCREEN,
        SWP_NOZORDER, SWP_NOACTIVATE, SWP_NOSIZE,
        WM_SETICON, ICON_BIG, ICON_SMALL,
        IMAGE_ICON, LR_DEFAULTSIZE,
        GetWindowRect,
    };

    /// Resource ID for the icon embedded by winresource in build.rs.
    /// MAKEINTRESOURCE(1) = 1 as usize cast to pointer.
    const ICON_RESOURCE_ID: u16 = 1;

    /// Check if a HANDLE value is invalid (null or INVALID_HANDLE_VALUE).
    fn is_invalid_handle(h: *mut core::ffi::c_void) -> bool {
        h.is_null() || h == -1_isize as *mut _
    }

    pub fn setup() {
        let existing_console = unsafe { GetConsoleWindow() };

        if existing_console.is_null() {
            // Release mode with windows_subsystem = "windows": no console exists
            unsafe {
                if AllocConsole() == 0 {
                    return;
                }
            }
        }

        set_title();
        set_icon();
        configure_console_font();
        set_window_size(120, 35);
        center_window();
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

        // Load the icon from the embedded resource (ID 1, set by winresource).
        // MAKEINTRESOURCE(id) = id as usize as *const u16
        let hicon = unsafe {
            LoadImageW(
                hmodule,
                ICON_RESOURCE_ID as usize as *const u16,
                IMAGE_ICON,
                0,
                0,
                LR_DEFAULTSIZE,
            )
        };

        if hicon.is_null() {
            return;
        }

        unsafe {
            SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, hicon as isize);
            SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, hicon as isize);
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
