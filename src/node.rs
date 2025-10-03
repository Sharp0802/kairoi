use dashmap::DashMap;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::AcqRel;
use std::sync::{Arc, Weak};
use parking_lot::Mutex;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Hash)]
pub struct Id(usize);

impl Id {
    fn new() -> Self {
        static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
        Self(ID_COUNTER.fetch_add(1, AcqRel))
    }
}

pub struct Node<T: Send + Sync> {
    id: Id,
    value: Mutex<Arc<T>>,
    depth: usize,
    parent: Weak<Node<T>>,
    children: DashMap<Id, Arc<Node<T>>>,
}

impl<T: Send + Sync> Node<T> {
    pub fn new(value: T) -> Self {
        Self {
            id: Id::new(),
            value: Mutex::new(Arc::new(value)),
            depth: 0,
            parent: Weak::new(),
            children: DashMap::new(),
        }
    }


    pub fn id(&self) -> Id {
        self.id
    }

    pub fn value(&self) -> Arc<T> {
        self.value.lock().clone()
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn parent(&self) -> Option<Arc<Self>> {
        self.parent.upgrade()
    }

    pub fn children(&self) -> Vec<Arc<Self>> {
        self.children.iter().map(|v| v.deref().clone()).collect()
    }


    pub fn find<F: Fn(&Self) -> bool>(self: &Arc<Self>, f: F) -> Option<Arc<Self>> {
        if f(self) {
            return Some(self.clone());
        }

        for item in self.children.iter() {
            if let Some(found) = item.find(&f) {
                return Some(found);
            }
        }

        None
    }

    fn find_all_into<F: Fn(&Self) -> bool>(self: &Arc<Self>, f: F, v: &mut Vec<Arc<Self>>) {
        if f(self) {
            v.push(self.clone());
        }

        for item in self.children.iter() {
            item.find_all_into(&f, v);
        }
    }

    pub fn find_all<F: Fn(&Self) -> bool>(self: &Arc<Self>, f: F) -> Vec<Arc<Self>> {
        let mut v: Vec<Arc<Self>> = Vec::new();
        self.find_all_into(f, &mut v);
        v
    }


    pub fn add(self: &Arc<Self>, mut child: Self) -> Arc<Self> {
        child.parent = Arc::downgrade(self);
        child.depth = self.depth + 1;
        let child_ref = Arc::new(child);
        self.children.insert(child_ref.id, child_ref.clone());
        child_ref
    }

    pub fn update(&self, value: T) {
        *self.value.lock().deref_mut() = Arc::new(value);
    }

    pub fn delete(&self) {
        if let Some(parent) = self.parent.upgrade() {
            parent.children.remove(&self.id);
        }
    }
}
