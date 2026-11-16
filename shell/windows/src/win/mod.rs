//! The shell's calls into Windows. Every unsafe block of the shell lives in this folder.
pub mod born;
pub mod credentials;
pub mod job;
pub mod known_folders;
pub mod local_zone;
pub mod locale;
pub mod menu;
pub mod probe;
pub mod registry;
pub mod run_key;
pub mod taskbar;
pub mod tray_picture;
