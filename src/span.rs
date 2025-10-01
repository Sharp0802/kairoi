use crate::event::Event;
use lazy_static::lazy_static;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::atomic::Ordering::AcqRel;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::SystemTime;
use tokio::task_local;

#[derive(Debug, Clone)]
pub struct SpanData {
    timestamp: SystemTime,
    name: String,
    progress: Option<f32>,
}

impl SpanData {
    pub fn with_name(&self, name: String) -> Self {
        let mut clone = self.clone();
        clone.name = name;
        clone
    }

    pub fn with_progress(&self, progress: f32) -> Self {
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

    pub fn progress(&self) -> Option<f32> {
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
    children: Arc<Mutex<Vec<SpanRef>>>,
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
            children: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn scope_with<T, F: AsyncFnOnce(SpanScope) -> T>(parent: &SpanRef, f: F) -> T {
        let new = Span::new();
        let id = new.id;
        let new_ref = SpanRef::from(new);

        let mut guard = parent.0.children.lock().unwrap();
        let i = guard.len();
        guard.push(new_ref.clone());
        drop(guard);

        Event::span_begin(id).submit();

        Self::set_changed(true);

        let v = CURRENT
            .scope(RefCell::new(new_ref.downgrade()), async move {
                f(SpanScope::new(id)).await
            })
            .await;

        let mut guard = parent.0.children.lock().unwrap();
        guard.remove(i);
        drop(guard);

        Event::span_end(id).submit();

        drop(new_ref);

        v
    }

    pub fn children(&self) -> impl Iterator<Item = Span> {
        self.children
            .lock()
            .unwrap()
            .clone()
            .into_iter()
            .map(|v| v.0.deref().clone())
    }

    pub fn id(&self) -> SpanId {
        self.id
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

    fn changed() -> bool {
        CHANGED.load(Ordering::Acquire)
    }

    fn set_changed(changed: bool) {
        CHANGED.store(changed, Ordering::Release);
    }

    pub(crate) fn get_root(container: &mut Span) {
        if Self::changed() {
            *container = ROOT.0.deref().clone()
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
