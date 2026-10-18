#![no_std]

extern crate alloc;

mod commands;
mod editor;
mod input;
mod parser;
mod session;
mod terminal;

pub use session::run;
pub use terminal::Console;
