#![no_std]

mod pausable;
mod storage;

pub use crate::{
    pausable::{emit_paused, emit_unpaused, Pausable, PausableClient},
    storage::{pause, paused, unpause, when_not_paused, when_paused},
};

mod test;
