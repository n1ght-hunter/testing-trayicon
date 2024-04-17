use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize, Position},
    error::EventLoopError,
    event::{Event, WindowEvent},
    event_loop::{EventLoop, EventLoopWindowTarget},
    keyboard::KeyCode,
    raw_window_handle::{HasWindowHandle, RawWindowHandle, WindowHandle},
    tray::{Tray, TrayBuilder},
    window::{Icon, Window, WindowBuilder, WindowId},
};

fn main() -> Result<(), impl std::error::Error> {
    let event_loop = EventLoop::new().unwrap();

    // let mut window = Some(create_window(&event_loop)?);
    let mut window = None;

    let image = image::open("assets/rustacean.png").unwrap().into_rgba8();
    let (width, height) = image.dimensions();
    let container = image.into_raw();
    let icon = Icon::from_rgba(container, width, height).unwrap();

    let tray = TrayBuilder::new()
        .with_icon(icon)
        .with_tooltip("Hello, World!")
        .build(&event_loop)?;

    let mut current_position = PhysicalPosition::default();

    let mut notification: Option<Window> = Option::None;

    event_loop.run(move |event, elwt| match event {
        Event::WindowEvent { event, window_id } => {
            if notification.is_some() && window_id == notification.as_ref().unwrap().id() {
                match event {
                    WindowEvent::Focused(false) => {
                        notification = None;
                    }
                    _ => (),
                }
            }
            if window_id == tray.id() {
                match event {
                    WindowEvent::CursorMoved {
                        device_id: _,
                        position,
                    } => {
                        current_position = position;
                    }
                    WindowEvent::MouseInput {
                        device_id: _,
                        state,
                        button,
                    } => {
                        println!("spawn notification");
                        if state == winit::event::ElementState::Released
                            && notification.is_none()
                            && button == winit::event::MouseButton::Right
                        {
                            notification = spawn_notification(elwt, current_position).into();
                        }
                        if button == winit::event::MouseButton::Middle
                            && state == winit::event::ElementState::Released
                        {
                            elwt.exit();
                        }
                        if button == winit::event::MouseButton::Left
                            && state == winit::event::ElementState::Released
                        {
                            window = Some(create_window(&elwt).unwrap());
                        }
                    }
                    _ => (),
                }
            }
            if let Some(w) = &window {
                if window_id == w.id() {
                    match event {
                        WindowEvent::CloseRequested => {
                            window = None;
                        }
                        WindowEvent::KeyboardInput { event, .. } => {
                            let key = event.physical_key;
                            if let winit::keyboard::PhysicalKey::Code(KeyCode::Escape) = key {
                                elwt.exit();
                            }
                        }
                        _ => (),
                    }
                }
            }
        }
        Event::AboutToWait => {
            if let Some(window) = &window {
                window.request_redraw();
            }
        }

        _ => (),
    })
}

fn create_window(event_loop: &EventLoopWindowTarget<()>) -> Result<Window, winit::error::OsError> {
    WindowBuilder::new()
        .with_title("A fantastic window!")
        .with_inner_size(winit::dpi::LogicalSize::new(128.0, 128.0))
        .build(&event_loop)
}

fn spawn_notification(
    event_loop: &EventLoopWindowTarget<()>,
    mouse_position: PhysicalPosition<f64>,
) -> Window {
    println!("mouse_position: {:?}", mouse_position);
    let width = 200.0;
    let height = 200.0;
    let pos = PhysicalPosition::new(mouse_position.x - width, mouse_position.y - height);
    let child_window = WindowBuilder::new()
        .with_title("child window")
        .with_inner_size(PhysicalSize::new(width, height))
        .with_position(pos)
        .with_decorations(false)
        .with_resizable(false)
        .with_window_level(winit::window::WindowLevel::AlwaysOnTop)
        .build(event_loop)
        .unwrap();

    child_window.focus_window();

    child_window
}
