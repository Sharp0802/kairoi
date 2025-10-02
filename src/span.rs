use crate::event::Event;
use dashmap::DashMap;
use lazy_static::lazy_static;
use parking_lot::MutexGuard;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::atomic::Ordering::{AcqRel, Relaxed, Release};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Weak};
use std::time::SystemTime;
use tokio::task_local;

#[derive(Debug, Copy, Clone)]
pub struct Progress {
    total: u64,
    progress: u64,
}

impl Progress {
    pub fn new(total: u64, progress: u64) -> Self {
        Self { total, progress }
    }

    pub fn total(&self) -> u64 {
        self.total
    }

    pub fn progress(&self) -> u64 {
        self.progress
    }
}

#[derive(Debug, Clone)]
pub struct SpanData {
    timestamp: SystemTime,
    name: String,
    progress: Option<Progress>,
}

impl SpanData {
    pub fn with_name(&self, name: String) -> Self {
        let mut clone = self.clone();
        clone.name = name;
        clone
    }

    pub fn with_progress(&self, progress: Progress) -> Self {
        let mut clone = self.clone();
        clone.progress = Some(progress);
        clone
    }

    pub fn timestamp(&self) -> SystemTime {
        self.timestamp
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn progress(&self) -> Option<Progress> {
        self.progress
    }
}

impl Default for SpanData {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now(),
            name: String::default(),
            progress: None,
        }
    }
}

#[derive(Clone)]
pub struct Span {
    id: SpanId,
    children: Arc<DashMap<SpanId, SpanRef>>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct SpanId(u64);

#[derive(Clone)]
struct SpanRef(Arc<Span>);

impl SpanRef {
    fn downgrade(&self) -> SpanWeakRef {
        SpanWeakRef(Arc::downgrade(&self.0))
    }
}

impl From<Span> for SpanRef {
    fn from(span: Span) -> Self {
        Self(Arc::new(span))
    }
}

#[derive(Clone)]
struct SpanWeakRef(Weak<Span>);

impl SpanWeakRef {
    fn upgrade(&self) -> Option<SpanRef> {
        self.0.upgrade().map(SpanRef)
    }
}

static CHANGED: AtomicBool = AtomicBool::new(true);

lazy_static! {
    static ref ROOT: SpanRef = SpanRef::from(Span::new());
}

task_local! {
    static CURRENT: RefCell<SpanWeakRef>;
}

impl Span {
    pub(crate) fn new() -> Self {
        static ID: AtomicU64 = AtomicU64::new(0);

        Self {
            id: SpanId(ID.fetch_add(1, AcqRel)),
            children: Arc::new(DashMap::new()),
        }
    }

    async fn scope_with<T, F: AsyncFnOnce(SpanScope) -> T>(parent: &SpanRef, f: F) -> T {
        let new = Span::new();
        let id = new.id;
        let new_ref = SpanRef::from(new);

        parent.0.children.insert(id, new_ref.clone()).unwrap();
        Event::span_begin(id).submit();

        CHANGED.store(true, Release);

        let v = CURRENT
            .scope(RefCell::new(new_ref.downgrade()), async move {
                f(SpanScope::new(id)).await
            })
            .await;

        parent.0.children.remove(&id).unwrap();
        Event::span_end(id).submit();

        drop(new_ref);

        v
    }

    pub fn children(&self) -> Vec<Span> {
        self.children.iter().map(|v| v.0.deref().clone()).collect() // must collect: any reference to item of map can cause deadlock
    }

    pub fn id(&self) -> SpanId {
        self.id
    }

    fn find_depth_impl(&self, id: SpanId, depth: usize) -> Option<usize> {
        if self.id == id {
            return Some(depth);
        }

        if self.children.contains_key(&id) {
            return Some(depth + 1);
        }

        for x in self.children.iter() {
            if let Some(v) = x.0.find_depth_impl(id, depth + 1) {
                return Some(v);
            }
        }

        None
    }

    pub fn find_depth(&self, id: SpanId) -> Option<usize> {
        self.find_depth_impl(id, 0)
    }

    pub async fn scope<T, F: AsyncFnOnce(SpanScope) -> T>(f: F) -> T {
        let current = CURRENT.try_with(|v| {
            let cell = v.borrow_mut();
            return cell.upgrade().expect("CURRENT expired");
        });
        match current {
            Ok(current) => Self::scope_with(&current, f).await,
            Err(_) => Self::scope_with(&ROOT, f).await,
        }
    }

    pub fn current() -> SpanId {
        CURRENT
            .try_with(|v| v.borrow().upgrade().expect("CURRENT expired").0.id)
            .unwrap_or_else(|_| ROOT.0.id)
    }

    pub(crate) fn get_root(container: &mut Span) {
        if CHANGED
            .compare_exchange(true, false, AcqRel, Relaxed)
            .is_ok()
        {
            *container = ROOT.0.deref().clone();
        }
    }
}

pub struct SpanScope {
    _marker: PhantomData<MutexGuard<'static, ()>>,
    id: SpanId,
}

impl SpanScope {
    fn new(id: SpanId) -> Self {
        Self {
            _marker: Default::default(),
            id,
        }
    }

    pub fn update(&self, data: SpanData) {
        Event::span(self.id, data).submit()
    }
}
