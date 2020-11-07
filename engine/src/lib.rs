#![warn(clippy::all, clippy::nursery, clippy::pedantic, missing_docs)]

//! # Engine
//! 
//! A library for doing all engine related tasks of a voxel based game or renderer

/// # Ticket
/// 
/// The module for dealing with where to load chunks, what chunks to update, and which data to send to rendering
pub mod ticket;

/// # Clock
/// 
/// The module for dealing with controlling the in engine clock to be able to not be bound by any hardware factor such as cpu speed of frame-rate
pub mod clock;

/// # Input
/// 
/// The module for dealing with all input related tasks
pub mod input;