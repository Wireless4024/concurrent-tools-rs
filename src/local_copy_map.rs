use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{BuildHasher, Hash, RandomState};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::LocalKey;

use downcast::{Any, downcast};
use dyn_clone::{clone_trait_object, DynClone};

/// LocalCopyMap is a thread local map that allow you to store data in global map and when access,
/// value will be copied to thread local map.
/// # Note
/// - This structure are not guarantee that value will be copied to thread local map after updated globally.
///   This is useful to avoid frequent clone from global to local.
/// - [K] and [V] must be `Send + Sync` and `Clone`
/// # Example
/// ```rust
/// use concurrent_tools::local_copy_map::{LocalCopyMap, LocalMapType};
/// use std::collections::HashMap;
/// use std::hash::{BuildHasherDefault, RandomState};
/// use std::thread_local;
///
/// thread_local! {
///     static LOCAL: LocalMapType<i32, i32, RandomState> = LocalCopyMap::init_local(64); // capacity
/// }
/// let mut map = LocalCopyMap::new(64, &LOCAL);
/// map.set(1, 1);
/// assert_eq!(map.get(&1,|&value|value), Some(1));
/// ```
///
pub struct LocalCopyMap<K, V, L: 'static, S: BuildHasher + 'static = RandomState> {
    global: Vec<GlobalMap<K, V, S>>,
    hasher: S,
    local: &'static LocalKey<Vec<RefCell<L>>>,
}
pub type LocalMapType<K, V, S> = Vec<RefCell<LocalMap<K, V, S>>>;
pub type LocalAnyMap<S> = LocalCopyMap<TypeId, DynAnyClone, LocalMap<TypeId, DynAnyClone, S>, S>;
pub type LocalAnyMapType<S> = Vec<RefCell<LocalMap<TypeId, DynAnyClone, S>>>;

impl<K, V, S> LocalCopyMap<K, V, LocalMap<K, V, S>, S>
where
    S: BuildHasher + Default + Clone + 'static,
{
    pub fn new(capacity: usize, local: &'static LocalKey<Vec<RefCell<LocalMap<K, V, S>>>>) -> Self {
        let capacity = capacity.next_power_of_two();
        let mut global = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            global.push(GlobalMap::default());
        }
        Self {
            global,
            hasher: S::default(),
            local,
        }
    }
}
impl<S> LocalCopyMap<TypeId, DynAnyClone, LocalMap<TypeId, DynAnyClone, S>, S>
where
    S: BuildHasher + Default + Clone + 'static,
{
    /// Helper init function to create any map
    pub fn new_any_map(capacity: usize, local: &'static LocalKey<Vec<RefCell<LocalMap<TypeId, DynAnyClone, S>>>>) -> Self {
        Self::new(capacity, local)
    }

    /// Helper init function to create any map in thread local macro
    pub fn init_any_local(capacity: usize) -> Vec<RefCell<LocalMap<TypeId, DynAnyClone, S>>> {
        let capacity = capacity.next_power_of_two();
        let mut local = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            local.push(RefCell::new(LocalMap::default()));
        }
        local
    }

    pub fn set_any<T: AnyClone>(&self, value: T) -> Option<Box<T>> {
        let value = self.set(TypeId::of::<T>(), DynAnyClone::new(value));
        value.and_then(|value| value.inner.downcast().ok())
    }

    pub fn set_boxed<T: AnyClone>(&self, value: Box<T>) -> Option<Box<T>> {
        let value = self.set(TypeId::of::<T>(), DynAnyClone::new_boxed(value));
        value.and_then(|value| value.inner.downcast().ok())
    }

    pub fn get_downcast<T: 'static, F: FnOnce(&T) -> R + 'static, R: Send + 'static>(&self, callback: F) -> Option<R> {
        self.get(&TypeId::of::<T>(), |value| match value.inner.downcast_ref::<T>() {
            Ok(value) => {
                Some(callback(value))
            }
            Err(_) => {
                None
            }
        }).flatten()
    }
}
impl<K: Hash + Eq, V, S: BuildHasher + Default> LocalCopyMap<(), (), LocalMap<K, V, S>, S> {
    pub fn init_local(capacity: usize) -> Vec<RefCell<LocalMap<K, V, S>>> {
        let capacity = capacity.next_power_of_two();
        let mut local = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            local.push(RefCell::new(LocalMap::default()));
        }
        local
    }
}

impl<K, V, S, L> LocalCopyMap<K, V, L, S>
where
    K: Hash + Eq + Clone + 'static,
    V: Clone + 'static,
    S: BuildHasher + Default + Clone + 'static,
{
    pub fn set(&self, key: K, value: V) -> Option<V> {
        let hash = self.hasher.hash_one(&key);
        let mask = (self.global.len() - 1) as u64;
        let index = (hash & mask) as usize;
        let global = self.global.get(index)
            .expect("Global must contain at lease size of local");
        global.mod_count.fetch_add(1, Ordering::Relaxed);
        let mut map = global.map.lock().unwrap();
        map.insert(key, value)
    }
}

impl<K, V, S> LocalCopyMap<K, V, LocalMap<K, V, S>, S>
where
    K: Hash + Eq + Clone + 'static,
    V: Clone + 'static,
    S: BuildHasher + Default + Clone + 'static,
{
    pub fn get<F: FnOnce(&V) -> R + 'static, R: 'static>(&self, key: &K, callback: F) -> Option<R> {
        // capacity is (power of 2) - 1
        let mask = (self.global.len() - 1) as u64;
        let hash = self.hasher.hash_one(key);
        let index = (hash & mask) as usize;
        let global = &self.global[index];
        self.local.with(|local| {
            let cell = local.get(index)
                .expect("Local must contain at lease size of global");

            let mut map = cell.borrow_mut();

            // # Safety
            // Data race is acceptable and values will lag behind
            // this is useful to avoid frequent clone from global to local
            let expected = unsafe {
                *global.mod_count.as_ptr()
            };
            if map.mod_count != expected {
                map.clone_from(expected, &global.map);
            }
            map.map.get(key).map(callback)
        })
    }

    pub unsafe fn get_unchecked<F: FnOnce(&V) -> R + 'static, R: 'static>(&self, key: &K, callback: F) -> Option<R> {
        // capacity is (power of 2) - 1
        let mask = (self.global.len() - 1) as u64;
        let hash = self.hasher.hash_one(key);
        let index = (hash & mask) as usize;
        let global = self.global.get_unchecked(index);
        self.local.with(|local| {
            let cell = local.get_unchecked(index);

            let mut map = cell.borrow_mut();

            // # Safety
            // Data race is acceptable and values will lag behind
            // this is useful to avoid frequent clone from global to local
            let expected = *global.mod_count.as_ptr();
            if map.mod_count != expected {
                map.clone_from(expected, &global.map);
            }
            map.map.get(key).map(callback)
        })
    }
}

struct GlobalMap<K, V, S> {
    mod_count: AtomicUsize,
    map: Arc<Mutex<HashMap<K, V, S>>>,
}

impl<K, V, S: Default> Default for GlobalMap<K, V, S> {
    fn default() -> Self {
        Self {
            mod_count: AtomicUsize::new(0),
            map: Arc::default(),
        }
    }
}

/// this struct is !Send but store in thread local storage
pub struct LocalMap<K, V, S> {
    mod_count: usize,
    map: HashMap<K, V, S>,
}

impl<K: Hash + Eq, V, S: BuildHasher + Default> Default for LocalMap<K, V, S> {
    fn default() -> Self {
        Self {
            mod_count: 0,
            map: HashMap::default(),
        }
    }
}

impl<K: Hash + Eq + Clone, V: Clone, S: BuildHasher + Default + Clone> LocalMap<K, V, S> {
    #[inline(never)]
    fn clone_from(&mut self, mod_count: usize, other: &Mutex<HashMap<K, V, S>>) {
        if other.is_poisoned() {
            other.clear_poison();
        }
        let map = other.lock().unwrap();
        self.mod_count = mod_count;
        self.map.clone_from(&map);
    }
}

const fn send_sync<T: Send + Sync>() {}
const _: () = {
    send_sync::<GlobalMap<i32, i32, RandomState>>();
    send_sync::<LocalCopyMap<i32, i32, LocalMap<i32, *const (), RandomState>, RandomState>>();
};


pub trait AnyClone: DynClone + Any + Send {}
clone_trait_object!(AnyClone);
downcast!(dyn AnyClone);
impl<T: Clone + Any + Send> AnyClone for T {}

#[derive(Clone)]
pub struct DynAnyClone {
    inner: Box<dyn AnyClone>,
}

impl DynAnyClone {
    pub fn new<T: AnyClone>(value: T) -> Self {
        Self {
            inner: Box::new(value),
        }
    }

    pub fn new_boxed<T: AnyClone>(value: Box<T>) -> Self {
        Self {
            inner: value,
        }
    }
}