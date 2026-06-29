// src/platform/mod.rs - Cross-platform abstraction layer

pub mod app_launcher;
pub mod file_system;
pub mod process_builder;
pub mod system_control;

pub use app_launcher::{AppLauncher, AppLauncherImpl};
pub use file_system::{FileSystemProvider, FileSystemProviderImpl};
pub use process_builder::ProcessBuilderExt;
pub use system_control::{SystemController, get_system_controller};
