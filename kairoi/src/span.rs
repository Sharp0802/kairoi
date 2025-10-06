use crate::{Event, Node};
use lazy_static::lazy_static;
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
pub struct Span {
    timestamp: SystemTime,
    name: String,
    progress: Option<Progress>,
}

pub type SpanRef = Arc<Node<Span>>;

impl Span {
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

impl Default for Span {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now(),
            name: String::default(),
            progress: None,
        }
    }
}

lazy_static! {
    static ref ROOT: SpanRef = Arc::new(Node::new(Span::default()));
}

task_local! {
    static CURRENT: Weak<Node<Span >>;
}

impl Span {
    pub async fn scope<T, F: AsyncFnOnce(Scope) -> T>(f: F) -> T {
        let new = Self::current().add(Node::new(Self::default()));

        Event::span_begin(new.clone()).submit();

        let new_clone = new.clone();
        let v = CURRENT
            .scope(Arc::downgrade(&new_clone), async move {
                f(Scope::new(new_clone)).await
            })
            .await;

        new.delete();
        Event::span_end(new).submit();

        v
    }

    pub fn current() -> SpanRef {
        CURRENT
            .try_with(|v| v.upgrade())
            .ok()
            .flatten()
            .unwrap_or_else(|| ROOT.clone())
    }

    pub fn root() -> SpanRef {
        ROOT.clone()
    }
}

pub struct Scope {
    node: SpanRef,
}

impl Scope {
    fn new(node: SpanRef) -> Self {
        Self { node }
    }

    pub fn update(&self, data: Span) {
        self.node.update(data);
    }
}
