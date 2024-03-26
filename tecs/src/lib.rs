mod vecany;
pub use vecany::VecAny;

use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    marker::PhantomData,
    rc::Rc,
};

pub trait System<E> {
    fn event(&self, world: &mut World<E>, event: &E);
    fn tick(&self, world: &mut World<E>);
}

struct Handler<T>(T);
struct Ticker<T>(T);

impl<E, T: Fn(&mut World<E>, &E)> System<E> for Handler<T> {
    fn event(&self, world: &mut World<E>, event: &E) {
        self.0(world, event)
    }
    fn tick(&self, _: &mut World<E>) {}
}

impl<E, T: Fn(&mut World<E>)> System<E> for Ticker<T> {
    fn event(&self, _: &mut World<E>, _: &E) {}
    fn tick(&self, world: &mut World<E>) {
        self.0(world)
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

#[macro_export]
macro_rules! impl_archetype {
    (struct $for:ident { $( $field:ident: $type:ty ),* $(,)?}) => {
        concat_idents::concat_idents!(for_ref = $for, Ref {
            pub struct for_ref<'a> {
                $($field: std::cell::Ref<'a, $type>,)*
            }
        });

        concat_idents::concat_idents!(for_mut = $for, Mut {
            pub struct for_mut<'a> {
                $($field: std::cell::RefMut<'a, $type>,)*
            }
        });

        impl tecs::Archetype for $for {
            concat_idents::concat_idents!(for_ref = $for, Ref {
                type Ref<'a> = for_ref<'a>;

                fn from_components<'a>(components: &'a std::collections::HashMap<std::any::TypeId, std::cell::RefCell<tecs::VecAny>>, indices: &[u32]) -> for_ref<'a> {
                    let mut indices = indices.iter();

                    for_ref {
                        $($field: std::cell::Ref::map(components.get(&std::any::TypeId::of::<$type>()).unwrap().borrow(), |x| x.downcast_ref::<$type>().unwrap().get(*indices.next().unwrap() as usize).unwrap()),)*
                    }
                }
            });
            concat_idents::concat_idents!(for_mut = $for, Mut {
                type Mut<'a> = for_mut<'a>;

                fn from_components_mut<'a>(components: &'a std::collections::HashMap<std::any::TypeId, std::cell::RefCell<tecs::VecAny>>, indices: &[u32]) -> for_mut<'a> {
                    let mut indices = indices.iter();

                    for_mut {
                        $($field: std::cell::RefMut::map(components.get(&std::any::TypeId::of::<$type>()).unwrap().borrow_mut(), |x| x.downcast_mut::<$type>().unwrap().get_mut(*indices.next().unwrap() as usize).unwrap()),)*
                    }
                }
            });

            fn get_columns() -> Vec<std::any::TypeId> {
                vec![$(std::any::TypeId::of::<$type>()),*]
            }

            fn add_components(self, components: &mut std::collections::HashMap<std::any::TypeId, std::cell::RefCell<tecs::VecAny>>) {
                $(
                    components.get_mut(&std::any::TypeId::of::<$type>()).unwrap().borrow_mut().push(self.$field);
                )*
            }


        }
    };
}

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

pub struct World<E> {
    archetypes: HashMap<TypeId, ArchetypeTable>,
    components: HashMap<TypeId, RefCell<VecAny>>,
    systems: Vec<Rc<dyn System<E>>>,
    resources: HashMap<TypeId, Rc<RefCell<dyn Any>>>,
}

impl<E> Default for World<E> {
    fn default() -> Self {
        Self {
            archetypes: HashMap::new(),
            components: HashMap::new(),
            systems: Vec::new(),
            resources: HashMap::new(),
        }
    }
}

impl<E> World<E> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_system<T: System<E> + 'static>(mut self, system: T) -> Self {
        self.systems.push(Rc::new(system));
        self
    }

    pub fn with_handler<T: Fn(&mut World<E>, &E) + 'static>(mut self, handler: T) -> Self {
        self.systems.push(Rc::new(Handler(handler)));
        self
    }

    pub fn with_ticker<T: Fn(&mut World<E>) + 'static>(mut self, ticker: T) -> Self {
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

    pub fn get_components_mut<T: Any>(&self) -> RefMut<'_, [T]> {
        RefMut::map(
            self.components
                .get(&TypeId::of::<T>())
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

    pub fn get_entities_mut<'a, T: Archetype>(&'a self) -> Vec<T::Mut<'a>> {
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

    pub fn get_mut<T: Any>(&self) -> Option<RefMut<'_, T>> {
        self.resources
            .get(&TypeId::of::<T>())
            .map(|resource| RefMut::map(resource.borrow_mut(), |x| x.downcast_mut().unwrap()))
    }

    pub fn tick(&mut self) {
        self.systems
            .clone()
            .into_iter()
            .for_each(|system| system.tick(self))
    }

    pub fn submit(&mut self, event: E) {
        self.systems
            .clone()
            .into_iter()
            .for_each(|system| system.event(self, &event))
    }
}
