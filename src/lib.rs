#![deny(clippy::all)]
#![allow(clippy::field_reassign_with_default)]
#![feature(box_syntax)]
#![feature(crate_visibility_modifier)]
#![feature(try_trait)]
#![feature(wrapping_int_impl)]
#![warn(rust_2018_idioms)]

#[macro_use]
extern crate static_assertions;

pub mod convert;
pub mod header;
pub mod ipl3;
pub mod rom;
pub mod stream;
pub mod util;
