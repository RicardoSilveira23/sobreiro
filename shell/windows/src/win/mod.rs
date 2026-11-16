//! The shell's calls into Windows. Every unsafe block of the shell lives in this folder.
pub mod born;
pub mod credentials;
pub mod job;
pub mod known_folders;
pub mod local_zone;
pub mod probe;
pub mod registry;
pub mod taskbar;
