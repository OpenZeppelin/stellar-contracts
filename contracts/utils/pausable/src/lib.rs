#![no_std]

pub mod events;
mod pausable;
mod storage;

pub use crate::{
    pausable::{Pausable, PausableClient},
    storage::{pause, paused, unpause, when_not_paused, when_paused},
};

mod test;
