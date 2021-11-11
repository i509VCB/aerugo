pub mod watcher;

#[cfg(target_os = "linux")]
#[path = "./linux.rs"]
mod imp;

#[cfg(target_os = "linux")]
pub use imp::*;

#[cfg(not(target_os = "linux"))]
compile_error!("No config watcher implementation outside of linux at the moment.");
