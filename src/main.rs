use iced_core::window::Icon;
use std::{ffi::OsStr, mem, os::windows::ffi::OsStrExt, ptr};

use windows_sys::Win32::{
    Foundation::{HMODULE, HWND, LPARAM, LRESULT, POINT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Shell::{
            Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_MODIFY, NOTIFYICONDATAW,
        },
        WindowsAndMessaging::{
            CreateIcon, CreatePopupMenu, CreateWindowExW, DefWindowProcW, GetCursorPos, LoadIconW, PostQuitMessage,
            RegisterClassW, RegisterWindowMessageW, SetForegroundWindow, SetMenuInfo,
            CW_USEDEFAULT, HICON, HMENU, IDI_APPLICATION,
            MENUINFO, MIM_APPLYTOSUBMENUS, MIM_STYLE, MNS_NOTIFYBYPOS, WM_CREATE, WM_DESTROY, WM_LBUTTONUP, WM_MENUCOMMAND,
            WM_RBUTTONUP, WM_USER, WNDCLASSW, WS_OVERLAPPEDWINDOW,
        },
    },
};

fn main() {
    let image = image::open("assets/rustacean.png").unwrap().into_rgba8();
    let (width, height) = image.dimensions();
    let container = image.into_raw();
    let icon = iced_core::window::icon::from_rgba(container, width, height).unwrap();
    let window_info = unsafe { init_window() }.unwrap();

    window_info.set_tooltip("Hello, World!").unwrap();
    window_info.set_icon(icon).unwrap();

    loop {}
}

#[derive(Clone)]
pub struct WindowInfo {
    pub hwnd: HWND,
    pub hmodule: HMODULE,
    pub hmenu: HMENU,
}

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

impl WindowInfo {
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
        icon_data.hWnd = self.hwnd;
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
        nid.hWnd = self.hwnd;
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

unsafe impl Send for WindowInfo {}
unsafe impl Sync for WindowInfo {}

pub(crate) fn to_wstring(str: &str) -> Vec<u16> {
    OsStr::new(str)
        .encode_wide()
        .chain(Some(0).into_iter())
        .collect::<Vec<_>>()
}

pub(crate) unsafe fn init_window() -> Result<WindowInfo, ()> {
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
        0,
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

    Ok(WindowInfo {
        hwnd,
        hmenu,
        hmodule,
    })
}

pub(crate) unsafe extern "system" fn window_proc(
    h_wnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    static mut U_TASKBAR_RESTART: u32 = 0;

    if msg == WM_MENUCOMMAND {
        // WININFO_STASH.with(|stash| {
        //     let stash = stash.borrow();
        //     let stash = stash.as_ref();
        //     if let Some(stash) = stash {
        //         let menu_id = GetMenuItemID(stash.info.hmenu, w_param as i32) as i32;
        //         if menu_id != -1 {
        //             stash.tx.send(WindowsTrayEvent(menu_id as u32)).ok();
        //         }
        //     }
        // });
    }

    if msg == WM_USER + 1 && (l_param as u32 == WM_LBUTTONUP || l_param as u32 == WM_RBUTTONUP) {
        let mut point = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut point) == 0 {
            return 1;
        }

        SetForegroundWindow(h_wnd);

        // WININFO_STASH.with(|stash| {
        //     let stash = stash.borrow();
        //     let stash = stash.as_ref();
        //     if let Some(stash) = stash {
        //         TrackPopupMenu(
        //             stash.info.hmenu,
        //             TPM_LEFTBUTTON | TPM_BOTTOMALIGN | TPM_LEFTALIGN,
        //             point.x,
        //             point.y,
        //             0,
        //             h_wnd,
        //             ptr::null(),
        //         );
        //     }
        // });
    }

    if msg == WM_CREATE {
        U_TASKBAR_RESTART = RegisterWindowMessageW(to_wstring("TaskbarCreated").as_ptr());
    }

    // If windows explorer restarts and we need to recreate the tray icon
    if msg == U_TASKBAR_RESTART {
        let icon: HICON = unsafe {
            let mut handle = LoadIconW(
                GetModuleHandleW(std::ptr::null()),
                to_wstring("tray-default").as_ptr(),
            );
            if handle == 0 {
                handle = LoadIconW(0, IDI_APPLICATION);
            }
            if handle == 0 {
                println!("Error setting icon from resource");
                PostQuitMessage(0);
            }
            handle as HICON
        };
        let mut nid = unsafe { mem::zeroed::<NOTIFYICONDATAW>() };
        nid.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = h_wnd;
        nid.uID = 1;
        nid.uFlags = NIF_MESSAGE | NIF_ICON;
        nid.hIcon = icon;
        nid.uCallbackMessage = WM_USER + 1;
        if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
            println!("Error adding menu icon");
            PostQuitMessage(0);
        }
    }

    if msg == WM_DESTROY {
        PostQuitMessage(0);
    }

    DefWindowProcW(h_wnd, msg, w_param, l_param)
}
