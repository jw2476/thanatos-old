use std::sync::Arc;

use winit::{
    event::WindowEvent, event_loop::EventLoop, platform::run_on_demand::EventLoopExtRunOnDemand,
    window::WindowBuilder,
};

use crate::{event::Event, world::World};

pub struct Window {
    event_loop: EventLoop<()>,
    pub window: Arc<winit::window::Window>,
}

impl Window {
    pub fn new() -> Self {
        let event_loop = EventLoop::new().unwrap();
        let window = WindowBuilder::new().build(&event_loop).unwrap();
        let window = Arc::new(window);
        Self { event_loop, window }
    }
}

pub fn poll_events(world: &mut World) {
    let mut events = Vec::new();
    {
        let mut window = world.get_mut::<Window>().unwrap();
        window
            .event_loop
            .run_on_demand(|event, control| {
                control.exit();

                if let winit::event::Event::WindowEvent {
                    window_id: _,
                    event,
                } = event
                {
                    match event {
                        WindowEvent::Resized(new_size) => {
                            events.push(Event::Resized(new_size));
                        }
                        WindowEvent::CloseRequested => {
                            events.push(Event::Stop);
                        }
                        _ => (),
                    }
                }
            })
            .unwrap();
    }

    events.into_iter().for_each(|event| world.submit(event));
}
