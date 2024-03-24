use glam::Vec2;
pub use winit::event::MouseButton;
pub use winit::keyboard::Key;

#[derive(Clone, Debug)]
pub enum Event {
    Resized(winit::dpi::PhysicalSize<u32>),
    Stop,
    KeyPress(Key),
    KeyRelease(Key),
    MousePress(MouseButton),
    MouseRelease(MouseButton),
    MouseMove { position: Vec2, delta: Vec2 }
}

