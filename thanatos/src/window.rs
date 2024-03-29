use std::{collections::HashSet, sync::Arc};

use glam::Vec2;
use winit::{
    event::{ElementState, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, SmolStr},
    platform::run_on_demand::EventLoopExtRunOnDemand,
    window::WindowBuilder,
};

use crate::{event::Event, World};

#[derive(Clone, Default)]
pub struct Mouse {
    pub position: Vec2,
    pub delta: Vec2,
}

pub fn clear_mouse_delta(world: &mut World) {
    let mut mouse = world.get_mut::<Mouse>().unwrap();
    mouse.delta = Vec2::ZERO;
}

#[derive(Clone, Default)]
pub struct Keyboard {
    down: HashSet<Key>,
}

impl Keyboard {
    pub fn is_down<T: IntoKey>(&self, key: T) -> bool {
        self.down.get(&key.into_key()).is_some()
    }
}

pub trait IntoKey {
    fn into_key(self) -> Key;
}

impl IntoKey for &str {
    fn into_key(self) -> Key {
        Key::Character(SmolStr::new_inline(self))
    }
}

impl IntoKey for Key {
    fn into_key(self) -> Key {
        self
    }
}

pub struct Window {
    event_loop: EventLoop<()>,
    pub window: Arc<winit::window::Window>,
}

impl Window {
    pub fn new() -> Self {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
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
                        WindowEvent::KeyboardInput { event, .. } => {
                            let mut keyboard = world.get_mut::<Keyboard>().unwrap();

                            match event.state {
                                ElementState::Pressed => {
                                    keyboard.down.insert(event.logical_key.clone());
                                    events.push(Event::KeyPress(event.logical_key));
                                }
                                ElementState::Released => {
                                    keyboard.down.remove(&event.logical_key);
                                    events.push(Event::KeyRelease(event.logical_key));
                                }
                            }
                        }
                        WindowEvent::MouseInput { state, button, .. } => match state {
                            ElementState::Pressed => events.push(Event::MousePress(button)),
                            ElementState::Released => events.push(Event::MouseRelease(button)),
                        },
                        WindowEvent::CursorMoved { position, .. } => {
                            let mut mouse = world.get_mut::<Mouse>().unwrap();
                            let position = Vec2::new(position.x as f32, position.y as f32);
                            mouse.delta = position - mouse.position;
                            mouse.position = position;
                            events.push(Event::MouseMove {
                                position,
                                delta: mouse.delta,
                            })
                        }
                        _ => (),
                    }
                }
            })
            .unwrap();
    }

    events.into_iter().for_each(|event| world.submit(event));
}
