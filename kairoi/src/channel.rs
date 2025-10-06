use crate::event::Event;
use crossbeam_channel::{bounded, Receiver, Sender};
use lazy_static::lazy_static;

lazy_static! {
    static ref CH: (Sender<Event>, Receiver<Event>) = bounded::<Event>(2048);
}

pub(crate) fn tx() -> &'static Sender<Event> {
    &CH.0
}

pub(crate) fn rx() -> &'static Receiver<Event> {
    &CH.1
}
