use crossbeam_channel::{unbounded, Receiver, Sender};
use lazy_static::lazy_static;
use crate::event::Event;

lazy_static! {
    static ref CH: (Sender<Event>, Receiver<Event>) = unbounded::<Event>();
}

pub(crate) fn tx() -> &'static Sender<Event> {
    &CH.0
}

pub(crate) fn rx() -> &'static Receiver<Event> {
    &CH.1
}
