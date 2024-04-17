use std::time::Duration;

use winit::{
    error::EventLoopError,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    raw_window_handle::{HasWindowHandle, RawWindowHandle, WindowHandle},
    tray::{Tray, TrayBuilder},
    window::{Icon, Window, WindowBuilder, WindowId},
};

use testing_trayicon::WindowsIconHandler;

fn main() -> Result<(), impl std::error::Error> {
    let event_loop = EventLoop::new().unwrap();

    let window = WindowBuilder::new()
        .with_title("A fantastic window!")
        .with_inner_size(winit::dpi::LogicalSize::new(128.0, 128.0))
        .build(&event_loop)
        .unwrap();

    let image = image::open("assets/rustacean.png").unwrap().into_rgba8();
    let (width, height) = image.dimensions();
    let container = image.into_raw();
    let icon = Icon::from_rgba(container, width, height).unwrap();

    let tray = TrayBuilder::new()
        .with_icon(icon)
        .with_tooltip("Hello, World!")
        .build(&event_loop)?;

    event_loop.run(move |event, elwt| {
        // println!("{event:?}");

        match event {
            Event::WindowEvent { event, window_id } => {
                if window_id != window.id() {
                    println!("{event:?}");
                }
                if window_id == window.id() {
                    match event {
                        WindowEvent::CloseRequested => elwt.exit(),
                        _ => (),
                    }
                }
            }
            Event::AboutToWait => {
                window.request_redraw();
            }

            _ => (),
        }
    })
}
