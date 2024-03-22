use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    rc::Rc,
};

use crate::event::Event;

pub trait System {
    fn event(&self, world: &mut World, event: &Event);
    fn tick(&self, world: &mut World);
}

struct Handler<T>(T);
struct Ticker<T>(T);

impl<T: Fn(&mut World, &Event)> System for Handler<T> {
    fn event(&self, world: &mut World, event: &Event) {
        self.0(world, event)
    }
    fn tick(&self, _: &mut World) {}
}

impl<T: Fn(&mut World)> System for Ticker<T> {
    fn event(&self, _: &mut World, _: &Event) {}
    fn tick(&self, world: &mut World) {
        self.0(world)
    }
}

pub enum State {
    Stopped,
    Running,
}

fn handle_stop(world: &mut World, event: &Event) {
    if let Event::Stop = event {
        *world.get_mut::<State>().unwrap() = State::Stopped;
    }
}

#[derive(Default)]
pub struct World {
    systems: Vec<Rc<dyn System>>,
    resources: HashMap<TypeId, Rc<RefCell<dyn Any>>>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
            .with_resource(State::Stopped)
            .with_handler(handle_stop)
    }

    pub fn with_system<T: System + 'static>(mut self, system: T) -> Self {
        self.systems.push(Rc::new(system));
        self
    }

    pub fn with_handler<T: Fn(&mut World, &Event) + 'static>(mut self, handler: T) -> Self {
        self.systems.push(Rc::new(Handler(handler)));
        self
    }


    pub fn with_ticker<T: Fn(&mut World) + 'static>(mut self, ticker: T) -> Self {
        self.systems.push(Rc::new(Ticker(ticker)));
        self
    }

    pub fn with_resource<T: Any>(mut self, resource: T) -> Self {
        self.resources
            .insert(TypeId::of::<T>(), Rc::new(RefCell::new(resource)));
        self
    }

    pub fn get<T: Any>(&self) -> Option<Ref<'_, T>> {
        self.resources
            .get(&TypeId::of::<T>())
            .map(|resource| Ref::map(resource.borrow(), |x| x.downcast_ref().unwrap()))
    }

    pub fn get_mut<T: Any>(&mut self) -> Option<RefMut<'_, T>> {
        self.resources
            .get_mut(&TypeId::of::<T>())
            .map(|resource| RefMut::map(resource.borrow_mut(), |x| x.downcast_mut().unwrap()))
    }

    fn tick(&mut self) {
        self.systems
            .clone()
            .into_iter()
            .for_each(|system| system.tick(self))
    }

    pub fn run(mut self) {
        *self.get_mut::<State>().unwrap() = State::Running;
        loop {
            if let State::Stopped = *self.get_mut().unwrap() { break }
            self.tick()
        }
    }

    pub fn submit(&mut self, event: Event) {
        self.systems
            .clone()
            .into_iter()
            .for_each(|system| system.event(self, &event))
    }
}
