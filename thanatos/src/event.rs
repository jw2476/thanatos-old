#[derive(Clone, Debug)]
pub enum Event {
    Resized(winit::dpi::PhysicalSize<u32>),
    Stop
}


