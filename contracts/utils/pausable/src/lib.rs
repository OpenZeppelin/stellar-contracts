#![no_std]

pub mod clients;
pub mod errors;
pub mod events;
mod storage;

pub use crate::{
    clients::{Pausable, PausableClient},
    storage::{paused, pause, unpause, when_not_paused, when_paused},
};

mod test;
