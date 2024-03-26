use std::{alloc::Layout, any::TypeId};

pub struct VecAny {
    ptr: Option<*mut u8>,
    len: usize,
    cap: usize,
    id: TypeId,
}

impl VecAny {
    pub fn new<T: 'static>() -> Self {
        Self {
            ptr: unsafe { Some(std::alloc::alloc(Layout::new::<T>())) },
            len: 0,
            cap: 0,
            id: TypeId::of::<T>(),
        }
    }

    pub fn new_uninit(id: TypeId) -> Self {
        Self {
            ptr: None,
            len: 0,
            cap: 0,
            id,
        }
    }

    pub fn from_slice<T: 'static + Copy>(data: &[T]) -> Self {
        let mut vec = Self {
            ptr: unsafe { Some(std::alloc::alloc(Layout::new::<T>())) },
            len: data.len(),
            cap: data.len(),
            id: TypeId::of::<T>(),
        };
        vec.ptr = Some(unsafe {
            std::alloc::realloc(
                vec.ptr.unwrap(),
                Layout::new::<T>(),
                vec.cap * std::mem::size_of::<T>(),
            )
        });

        unsafe {
            std::slice::from_raw_parts_mut(vec.ptr.unwrap().cast(), vec.len).copy_from_slice(&data);
        }

        vec
    }

    pub fn downcast_ref<T: 'static>(&self) -> Option<&[T]> {
        if self.id != TypeId::of::<T>() {
            return None;
        }
        Some(unsafe { std::slice::from_raw_parts(self.ptr?.cast(), self.len) })
    }

    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut [T]> {
        if self.id != TypeId::of::<T>() {
            return None;
        }

        Some(unsafe { std::slice::from_raw_parts_mut(self.ptr?.cast(), self.len) })
    }

    pub fn push<T: 'static>(&mut self, item: T) {
        if self.ptr.is_none() {
            self.ptr = Some(unsafe { std::alloc::alloc(Layout::new::<T>()) })
        }

        if self.id != TypeId::of::<T>() {
            return;
        }

        if self.len == self.cap {
            self.cap *= 2;

            self.ptr = Some(unsafe {
                std::alloc::realloc(
                    self.ptr.unwrap(),
                    Layout::new::<T>(),
                    self.cap * std::mem::size_of::<T>(),
                )
            })
        }

        self.len += 1;
        unsafe {
            std::slice::from_raw_parts_mut(self.ptr.unwrap().cast(), self.len)[self.len - 1] = item;
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}
