//! Platform-specific code for Aurora Compass
//!
//! This crate handles platform-specific features for Windows, macOS, and Linux.

#![warn(missing_docs)]
#![warn(clippy::all)]

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;
