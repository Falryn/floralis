//! Tauri 命令模块
//!
//! 按领域拆分为 settings / games / mods / backup 四个子模块

pub mod settings;
pub mod games;
pub mod mods;
pub mod backup;

// Re-export all commands for generate_handler!
pub use settings::*;
pub use games::*;
pub use mods::*;
pub use backup::*;
