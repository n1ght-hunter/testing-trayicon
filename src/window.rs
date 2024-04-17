use std::{mem, ptr};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Shell::{
            Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIM_ADD, NOTIFYICONDATAW,
        },
        WindowsAndMessaging::{
            DefWindowProcW, GetCursorPos, LoadIconW, PostQuitMessage, RegisterWindowMessageW, SetForegroundWindow, HICON, IDI_APPLICATION, WM_CREATE, WM_DESTROY, WM_LBUTTONDBLCLK, WM_LBUTTONUP,
            WM_MBUTTONDBLCLK, WM_MBUTTONUP, WM_MENUCOMMAND, WM_RBUTTONDBLCLK, WM_RBUTTONUP, WM_USER, WM_XBUTTONDBLCLK, WM_XBUTTONUP,
        },
    },
};

use crate::{to_wstring, WindowsIconHandler};

#[derive(Debug)]
pub enum Click {
    Single(iced_core::mouse::Button),
    Double(iced_core::mouse::Button),
}

#[derive(Debug)]
pub struct TrayEvent {
    mouse: Click,
    position: (i32, i32),
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

    // if msg == WM_USER + 1 && l_param as u32 != WM_MOUSEMOVE {
    //     println!("l_param: {}", l_param);
    //     println!("w_param: {}", w_param);
    // }

    {
        let l_param = l_param as u32;

        if msg == WM_USER + 1
            && (l_param == WM_LBUTTONUP
                || l_param == WM_LBUTTONDBLCLK
                || l_param == WM_RBUTTONUP
                || l_param == WM_RBUTTONDBLCLK
                || l_param == WM_MBUTTONUP
                || l_param == WM_MBUTTONDBLCLK
                || l_param == WM_XBUTTONUP
                || l_param == WM_XBUTTONDBLCLK)
        {
            let mut point = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut point) == 0 {
                return 1;
            }

            let click = match l_param as u32 {
                WM_LBUTTONUP => Click::Single(iced_core::mouse::Button::Left),
                WM_RBUTTONUP => Click::Single(iced_core::mouse::Button::Right),
                WM_MBUTTONUP => Click::Single(iced_core::mouse::Button::Middle),
                WM_XBUTTONUP => Click::Single(iced_core::mouse::Button::Forward),
                WM_LBUTTONDBLCLK => Click::Double(iced_core::mouse::Button::Left),
                WM_RBUTTONDBLCLK => Click::Double(iced_core::mouse::Button::Right),
                WM_MBUTTONDBLCLK => Click::Double(iced_core::mouse::Button::Middle),
                WM_XBUTTONDBLCLK => Click::Double(iced_core::mouse::Button::Forward),
                _ => panic!("shouldnt be anything other than left or right click"),
            };

            let event = TrayEvent {
                mouse: click,
                position: (point.x, point.y),
            };

            println!("{:?}", event);

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