use glam::{Mat4, Vec3};

use crate::{event::Event, window::Window, World};

pub struct Camera {
    pub eye: Vec3,
    pub direction: Vec3,
    pub fov: f32,
    pub aspect: f32,
}

impl Camera {
    pub fn new(window: &Window) -> Self {
        let size = window.window.inner_size();
        let aspect = size.width as f32 / size.height as f32;
        Self {
            eye: Vec3::ONE * 3.0,
            direction: Vec3::NEG_ONE,
            fov: std::f32::consts::PI / 2.0,
            aspect,
        }
    }

    pub fn get_matrix(&self) -> Mat4 {
        let view = Mat4::look_to_rh(self.eye, self.direction, Vec3::Y);
        let projection = Mat4::perspective_infinite_rh(self.fov, self.aspect, 0.1);
        projection * view
    }
}

pub fn handle_resize(world: &mut World, event: &Event) {
    match event {
        Event::Resized(new_size) => {
            let mut camera = world.get_mut::<Camera>().unwrap();
            camera.aspect = new_size.width as f32 / new_size.height as f32;
        }
        _ => (),
    }
}
