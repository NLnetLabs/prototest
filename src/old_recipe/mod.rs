//! Recipes for programmatically generating data.
//!
//! A [`Recipe`] is a programmatic description of how to create binary data
//! from some input data. Typically, recipes are returned by specific
//! functions. They can be used as input to other functions to create
//! nested recipes. This module provides functions and helper types that
//! allow creating recipes for many use cases.
//!
//! When a recipe is _assembled,_ it is written into a [`Fragment`], which
//! is mostly a `Vec<u8>` with some convenience functions added.

pub use self::core::{Assemble, Recipe, Fragment};

pub mod core;
pub mod der;
