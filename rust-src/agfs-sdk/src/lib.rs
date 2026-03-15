//! AGFS SDK - Rust implementation
//!
//! This crate provides the core types and client for interacting with AGFS server.

#![warn(missing_docs)]
// TODO: Remove this allow once full implementation is complete
#![allow(dead_code)]

pub mod client;
pub mod error;
pub mod types;

pub use client::Client;
pub use error::AgfsError;
pub use types::*;
