//! Extracted minimum code from: https://github.com/redox-os/termion
//! The crate doesn't compile under wasm32-wasi, but this code does.
//! This is everything we need to make tui work over telnet.
//! License: MIT/X11.

pub mod clear;
pub mod color;
pub mod cursor;
pub mod style;
