pub mod app;
pub mod asr;
pub mod audio;
pub mod config;
pub mod inject;
pub mod model;

#[cfg(target_os = "windows")]
pub mod tray;
