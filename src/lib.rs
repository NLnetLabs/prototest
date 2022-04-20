//! A library of assorted tools for testing network protocol implementations.
//!
//! This library currently contains the following components:
//!
//! * [recipe]: allow programmatically generating binary payload data,
//! * [stream]: provides stream socket stand-ins that send and receive data
//!   according to some prescriped rules.
//!
pub mod recipe;
pub mod stream;
