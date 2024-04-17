mod window;

use iced_core::window::Icon;
use window::window_proc;
use std::{ffi::OsStr, mem, ops::Deref, os::windows::ffi::OsStrExt, ptr};

use windows_sys::Win32::{
    Foundation::{HMODULE, HWND, LPARAM, LRESULT, POINT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Shell::{
            Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
            NOTIFYICONDATAW,
        },
        WindowsAndMessaging::{
            CreateIcon, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DispatchMessageW,
            GetCursorPos, GetMessageW, LoadIconW, PostMessageW, PostQuitMessage, RegisterClassW,
            RegisterWindowMessageW, SetForegroundWindow, SetMenuInfo, TranslateMessage,
            CW_USEDEFAULT, HICON, IDI_APPLICATION, MENUINFO, MIM_APPLYTOSUBMENUS, MIM_STYLE,
            MNS_NOTIFYBYPOS, MSG, WM_CREATE, WM_DESTROY, WM_LBUTTONDBLCLK, WM_LBUTTONUP,
            WM_MBUTTONDBLCLK, WM_MBUTTONUP, WM_MENUCOMMAND, WM_MOUSEMOVE, WM_QUIT,
            WM_RBUTTONDBLCLK, WM_RBUTTONUP, WM_USER, WM_XBUTTONDBLCLK, WM_XBUTTONUP, WNDCLASSW,
            WS_OVERLAPPEDWINDOW,
        },
    },
};

#[repr(C)]
#[derive(Debug)]
pub(crate) struct Pixel {
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
    pub(crate) a: u8,
}

impl Pixel {
    fn convert_to_bgra(&mut self) {
        mem::swap(&mut self.r, &mut self.b);
    }
}

const PIXEL_SIZE: usize = std::mem::size_of::<Pixel>();

#[derive(Clone)]
pub struct WindowsIconHandler(HWND);

impl WindowsIconHandler {
    pub fn new() -> Result<WindowsIconHandler, ()> {
        unsafe { init_window(None) }
    }
    pub fn wiht_parent(parent_hwnd: HWND) -> Result<WindowsIconHandler, ()> {
        unsafe { init_window(Some(parent_hwnd)) }
    }

    pub fn set_icon(&self, icon: Icon) -> Result<(), &'static str> {
        let (rgba, size) = icon.into_raw();
        let pixel_count = rgba.len() / PIXEL_SIZE;
        let mut and_mask = Vec::with_capacity(pixel_count);

        let pixels =
            unsafe { std::slice::from_raw_parts_mut(rgba.as_ptr() as *mut Pixel, pixel_count) };
        //change rgba to bgra
        for pixel in pixels {
            and_mask.push(pixel.a.wrapping_sub(std::u8::MAX)); // invert alpha channel
            pixel.convert_to_bgra();
        }
        assert_eq!(and_mask.len(), pixel_count);
        let handle: HICON = unsafe {
            CreateIcon(
                0,
                size.width as i32,
                size.height as i32,
                1,
                (PIXEL_SIZE * 8) as u8,
                and_mask.as_ptr(),
                rgba.as_ptr(),
            )
        };
        if handle != 0 {
            self._set_icon(handle)
        } else {
            Err("Error creating icon")
        }
    }

    fn _set_icon(&self, icon: HICON) -> Result<(), &'static str> {
        let mut icon_data = unsafe { mem::zeroed::<NOTIFYICONDATAW>() };
        icon_data.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
        icon_data.hWnd = **self;
        icon_data.uID = 1;
        icon_data.uFlags = NIF_ICON;
        icon_data.hIcon = icon;

        unsafe {
            if Shell_NotifyIconW(NIM_MODIFY, &icon_data) == 0 {
                return Err("Error setting icon");
            }
        }
        Ok(())
    }

    pub fn set_tooltip(&self, tooltip: &str) -> Result<(), &'static str> {
        let wide_tooltip = to_wstring(tooltip);
        if wide_tooltip.len() > 128 {
            return Err("The tooltip may not exceed 127 wide bytes");
        }

        let mut nid = unsafe { mem::zeroed::<NOTIFYICONDATAW>() };
        nid.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = **self;
        nid.uID = 1;
        nid.uFlags = NIF_TIP;

        #[cfg(target_arch = "x86")]
        {
            let mut tip_data = [0u16; 128];
            tip_data[..wide_tooltip.len()].copy_from_slice(&wide_tooltip);
            nid.szTip = tip_data;
        }

        #[cfg(not(target_arch = "x86"))]
        nid.szTip[..wide_tooltip.len()].copy_from_slice(&wide_tooltip);

        unsafe {
            if Shell_NotifyIconW(NIM_MODIFY, &nid) == 0 {
                return Err("Error setting tooltip");
            }
        }
        Ok(())
    }
}

unsafe impl Send for WindowsIconHandler {}
unsafe impl Sync for WindowsIconHandler {}

impl Deref for WindowsIconHandler {
    type Target = HWND;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(crate) fn to_wstring(str: &str) -> Vec<u16> {
    OsStr::new(str)
        .encode_wide()
        .chain(Some(0).into_iter())
        .collect::<Vec<_>>()
}

pub(crate) unsafe fn run_loop() {
    // Run message loop
    let mut msg = unsafe { mem::zeroed::<MSG>() };
    loop {
        GetMessageW(&mut msg, 0, 0, 0);
        if msg.message == WM_QUIT {
            break;
        }
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}




pub unsafe fn init_window(parent_hwnd: Option<HWND>) -> Result<WindowsIconHandler, ()> {
    let hmodule = GetModuleHandleW(ptr::null());
    if hmodule == 0 {
        return Err(());
    }

    let class_name = to_wstring("my_window");

    let mut wnd = unsafe { mem::zeroed::<WNDCLASSW>() };
    wnd.lpfnWndProc = Some(window_proc);
    wnd.lpszClassName = class_name.as_ptr();

    RegisterClassW(&wnd);

    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        to_wstring("rust_systray_window").as_ptr(),
        WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT,
        0,
        CW_USEDEFAULT,
        0,
        parent_hwnd.unwrap_or(0) as HWND,
        0,
        0,
        ptr::null(),
    );
    if hwnd == 0 {
        return Err(());
    }

    let icon: HICON = unsafe {
        let mut handle = LoadIconW(
            GetModuleHandleW(std::ptr::null()),
            to_wstring("tray-default").as_ptr(),
        );
        if handle == 0 {
            handle = LoadIconW(0, IDI_APPLICATION);
        }
        if handle == 0 {
            return Err(());
        }
        handle as HICON
    };

    let mut nid = unsafe { mem::zeroed::<NOTIFYICONDATAW>() };
    nid.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON;
    nid.hIcon = icon;
    nid.uCallbackMessage = WM_USER + 1;

    if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
        return Err(());
    }

    // Setup menu
    let mut info = unsafe { mem::zeroed::<MENUINFO>() };
    info.cbSize = mem::size_of::<MENUINFO>() as u32;
    info.fMask = MIM_APPLYTOSUBMENUS | MIM_STYLE;
    info.dwStyle = MNS_NOTIFYBYPOS;
    let hmenu = CreatePopupMenu();
    if hmenu == 0 {
        return Err(());
    }
    if SetMenuInfo(hmenu, &info) == 0 {
        return Err(());
    }

    Ok(WindowsIconHandler(hwnd))
}