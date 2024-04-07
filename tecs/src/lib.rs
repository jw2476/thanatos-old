#![feature(impl_trait_in_assoc_type)]

mod vecany;
pub use vecany::VecAny;

use std::{
    any::{type_name, Any, TypeId}, cell::{Cell, Ref, RefCell, RefMut, UnsafeCell}, collections::HashMap, iter::Empty, marker::PhantomData, ops::{Deref, DerefMut, Index}, ptr::NonNull, rc::Rc
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
    fn columns() -> Vec<TypeId>;
    fn add(self, table: &mut Table);
}

#[macro_export]
macro_rules! impl_archetype {
    (struct $for:ident { $( $field:ident: $type:ty ),* $(,)?}) => {
        /*
        concat_idents::concat_idents!(for_ref = $for, Ref {
            pub struct for_ref<'a> {
                $($field: &'a $type,)*
            }
        });

        concat_idents::concat_idents!(for_mut = $for, Mut {
            pub struct for_mut<'a> {
                $($field: &'a mut $type,)*
            }
        });
*/

        impl tecs::Archetype for $for {
            /*
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
            */

            fn columns() -> Vec<std::any::TypeId> {
                vec![$(std::any::TypeId::of::<$type>()),*]
            }

            fn add(self, table: &mut tecs::Table) {
                table.length += 1;
                let mut columns = table.columns_mut();
                $(
                    columns.next().unwrap().push::<$type>(self.$field);
                )*
            }


        }
    };
}

pub struct RowIndex(u32);
pub struct Column {
    data: VecAny,
}

impl Column {
    pub fn new(ty: TypeId) -> Self {
        let data = VecAny::new_uninit(ty);
        Self { data }
    }

    pub fn get<T: 'static>(&self, index: RowIndex) -> Option<&T> {
        self.data.downcast_ref()?.get(index.0 as usize)
    }

    pub fn get_mut<T: 'static>(&mut self, index: RowIndex) -> Option<&mut T> {
        self.data.downcast_mut()?.get_mut(index.0 as usize)
    }

    pub fn push<T: 'static>(&mut self, item: T) {
        self.data.push(item)
    }
}

pub struct Table {
    pub length: usize,
    columns: Vec<(TypeId, RefCell<Column>)>,
}

impl Table {
    pub fn new(columns: &[TypeId]) -> Self {
        Self {
            length: 0,
            columns: columns.iter().cloned().map(|ty| (ty, RefCell::new(Column::new(ty)))).collect(),
        }
    }

    pub fn columns_mut(&self) -> impl Iterator<Item=RefMut<'_, Column>> {
        self.columns.iter().map(|(_, column)| column.borrow_mut())
    }

    pub fn column<T: 'static>(&self) -> Option<Ref<'_, [T]>> {
        self.columns
            .iter()
            .find(|(ty, _)| *ty == TypeId::of::<T>())
            .and_then(|(_, column)| Ref::filter_map(column.borrow(), |column| column.data.downcast_ref::<T>()).ok())
    }

    pub fn column_mut<T: 'static>(&self) -> Option<RefMut<'_, [T]>> {
        self.columns
            .iter()
            .find(|(ty, _)| *ty == TypeId::of::<T>())
            .and_then(|(_, column)| RefMut::filter_map(column.borrow_mut(), |column| column.data.downcast_mut::<T>()).ok())
    }

    pub fn len(&self) -> usize {
        self.length
    }
}

pub struct Columns<'a, T> {
    columns: Vec<Ref<'a, [T]>>
}

impl<'a, T> Columns<'a, T> {
    pub fn iter(&self) -> impl Iterator<Item=&T> {
        self.columns.iter().flat_map(|column| column.deref())
    } 
}

pub struct ColumnsMut<'a, T> {
    columns: Vec<RefMut<'a, [T]>>
}

impl<'a, T> ColumnsMut<'a, T> {
    pub fn iter(&self) -> impl Iterator<Item=&T> {
        self.columns.iter().flat_map(|column| column.deref())
    } 

    pub fn iter_mut(&'a mut self) -> impl Iterator<Item=&mut T> {
        self.columns.iter_mut().flat_map(|column| (*column).deref_mut().iter_mut())
    } 
}

pub trait Query<E> {
    type Output<'a>;

    fn query(tables: &HashMap<TypeId, Table>) -> Self::Output<'_>;
}

impl<T: 'static, E> Query<E> for &'_ T {
    type Output<'a> = Columns<'a, T>;

    fn query(tables: &HashMap<TypeId, Table>) -> Self::Output<'_> {
        let columns = tables.values().filter_map(|table| table.column::<T>()).collect();
        Columns { columns }
    }
}

impl<T: 'static, E> Query<E> for &'_ mut T {
    type Output<'a> = ColumnsMut<'a, T>;

    fn query(tables: &HashMap<TypeId, Table>) -> Self::Output<'_> {
        let columns = tables.values().filter_map(|table| table.column_mut::<T>()).collect();
        ColumnsMut { columns }
    }
}

impl<E, A: Query<E>, B: Query<E>> Query<E> for (A, B) {
    type Output<'a> = (A::Output<'a>, B::Output<'a>);

    fn query(tables: &HashMap<TypeId, Table>) -> Self::Output<'_> {
        (A::query(tables), B::query(tables))
    }
}

pub struct EntityId<T>(u32, PhantomData<T>);

pub struct World<E> {
    archetypes: HashMap<TypeId, Table>,
    systems: Vec<Rc<dyn System<E>>>,
    resources: HashMap<TypeId, Rc<RefCell<dyn Any>>>,
}

impl<E> Default for World<E> {
    fn default() -> Self {
        Self {
            archetypes: HashMap::new(),
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

    fn register<T: Archetype>(&mut self) {
        self.archetypes.insert(
            TypeId::of::<T>(),
            Table::new(&T::columns()),
        );
    }

    pub fn spawn<T: Archetype>(&mut self, entity: T) -> EntityId<T> {
        if !self.archetypes.contains_key(&TypeId::of::<T>()) {
            self.register::<T>();
        }

        let store = self
            .archetypes
            .get_mut(&TypeId::of::<T>())
            .unwrap();
        entity.add(store);
        EntityId(store.len() as u32 - 1, PhantomData)
    }

    pub fn query<Q: Query<E>>(&self) -> Q::Output<'_> {
        Q::query(&self.archetypes)
    }

    /*
    pub fn get_entities<T: Archetype>(&self) -> impl Iterator<Item = &T> {
        let Some(table) = self.archetypes.get(&TypeId::of::<T>()) else {
            return &[];
        };

        (0..table.borrow().len()).map(|index| T::from_components(components, indices))
    }

    pub fn get_entities_mut<T: Archetype>(&self) -> Vec<T::Mut<'_>> {
        let rows = &self.archetypes.get(&TypeId::of::<T>()).unwrap().rows;

        let mut output = Vec::new();
        for indices in rows {
            output.push(T::from_components_mut(&self.components, indices));
        }

        output
    }
    */

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

    pub fn remove<T: Any>(&mut self) -> Option<T> {
        self.resources.remove(&TypeId::of::<T>()).and_then(|rc| {
            let ptr: *const RefCell<dyn Any> = Rc::into_raw(rc);
            let ptr: *const RefCell<T> = ptr.cast();
            unsafe { Rc::into_inner(Rc::from_raw(ptr)).map(|x| x.into_inner()) }
        })
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
