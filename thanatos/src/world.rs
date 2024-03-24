use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    marker::PhantomData,
    rc::Rc,
};

use glam::Vec3;

use crate::{assets::MeshId, event::Event, structures::VecAny};

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

pub trait Archetype: Any {
    type Ref<'a>;
    type Mut<'a>;

    fn get_columns() -> Vec<TypeId>;
    fn add_components(self, components: &mut HashMap<TypeId, RefCell<VecAny>>);
    fn from_components<'a>(
        components: &'a HashMap<TypeId, RefCell<VecAny>>,
        indices: &[u32],
    ) -> Self::Ref<'a>;
    fn from_components_mut<'a>(
        components: &'a HashMap<TypeId, RefCell<VecAny>>,
        indices: &[u32],
    ) -> Self::Mut<'a>;
}

macro_rules! impl_archetype {
    ($for:ident, $for_ref:ident, $for_mut:ident, $( $field:ident: $type:ty ),*) => {
        pub struct $for {
            $($field: $type,)*
        }

        pub struct $for_ref<'a> {
            $($field: std::cell::Ref<'a, $type>,)*
        }


        pub struct $for_mut<'a> {
            $($field: std::cell::RefMut<'a, $type>,)*
        }

        impl crate::world::Archetype for $for {
            type Ref<'a> = $for_ref<'a>;
            type Mut<'a> = $for_mut<'a>;

            fn get_columns() -> Vec<std::any::TypeId> {
                vec![$(std::any::TypeId::of::<$type>()),*]
            }

            fn add_components(self, components: &mut std::collections::HashMap<std::any::TypeId, std::cell::RefCell<crate::structures::VecAny>>) {
                $(
                    components.get_mut(&std::any::TypeId::of::<$type>()).unwrap().borrow_mut().push(self.$field);
                )*
            }


            fn from_components<'a>(components: &'a std::collections::HashMap<std::any::TypeId, std::cell::RefCell<crate::structures::VecAny>>, indices: &[u32]) -> $for_ref<'a> {
                let mut indices = indices.iter();

                $for_ref {
                    $($field: std::cell::Ref::map(components.get(&std::any::TypeId::of::<$type>()).unwrap().borrow(), |x| x.downcast_ref::<$type>().unwrap().get(*indices.next().unwrap() as usize).unwrap()))*
                }
            }
            fn from_components_mut<'a>(components: &'a std::collections::HashMap<std::any::TypeId, std::cell::RefCell<crate::structures::VecAny>>, indices: &[u32]) -> $for_mut<'a> {
                let mut indices = indices.iter();

                $for_mut {
                    $($field: std::cell::RefMut::map(components.get(&std::any::TypeId::of::<$type>()).unwrap().borrow_mut(), |x| x.downcast_mut::<$type>().unwrap().get_mut(*indices.next().unwrap() as usize).unwrap()))*
                }
            }
        }
    };
}

pub(crate) use impl_archetype;

struct ArchetypeTable {
    columns: Vec<TypeId>,
    rows: Vec<Vec<u32>>,
}

impl ArchetypeTable {
    pub fn new(columns: Vec<TypeId>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
        }
    }
}

pub struct EntityId<T>(u32, PhantomData<T>);

#[derive(Default)]
pub struct World {
    archetypes: HashMap<TypeId, ArchetypeTable>,
    components: HashMap<TypeId, RefCell<VecAny>>,
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

    pub fn register<T: Archetype>(mut self) -> Self {
        let columns = T::get_columns();
        self.archetypes
            .insert(TypeId::of::<T>(), ArchetypeTable::new(columns.clone()));

        columns.into_iter().for_each(|column| {
            if self.components.get(&column).is_none() {
                self.components
                    .insert(column, RefCell::new(VecAny::new_uninit(column)));
            }
        });

        self
    }

    pub fn spawn<T: Archetype>(&mut self, entity: T) -> EntityId<T> {
        let columns = T::get_columns();
        let ids = columns
            .iter()
            .map(|column| self.components.get(column).unwrap().borrow().len() as u32)
            .collect::<Vec<u32>>();
        let store = self
            .archetypes
            .get_mut(&TypeId::of::<T>())
            .expect("Using unregistered archetype");
        store.rows.push(ids);
        entity.add_components(&mut self.components);
        EntityId(store.rows.len() as u32 - 1, PhantomData)
    }

    pub fn get_components<T: Any>(&self) -> Ref<'_, [T]> {
        Ref::map(
            self.components.get(&TypeId::of::<T>()).unwrap().borrow(),
            |x| x.downcast_ref().unwrap(),
        )
    }

    pub fn get_components_mut<T: Any>(&mut self) -> RefMut<'_, [T]> {
        RefMut::map(
            self.components
                .get_mut(&TypeId::of::<T>())
                .unwrap()
                .borrow_mut(),
            |x| x.downcast_mut().unwrap(),
        )
    }

    pub fn get_entities<T: Archetype>(&self) -> Vec<T::Ref<'_>> {
        self.archetypes
            .get(&TypeId::of::<T>())
            .unwrap()
            .rows
            .iter()
            .map(|indices| T::from_components(&self.components, indices))
            .collect()
    }

    pub fn get_entities_mut<'a, T: Archetype>(&'a mut self) -> Vec<T::Mut<'a>> {
        let rows = &self.archetypes.get(&TypeId::of::<T>()).unwrap().rows;

        let mut output = Vec::new();
        for indices in rows {
            output.push(T::from_components_mut(&self.components, indices));
        }

        output
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
            if let State::Stopped = *self.get_mut().unwrap() {
                break;
            }
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
