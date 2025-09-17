//! Modern Terminal UI for lrcget-cli
//!
//! This module provides a rich, interactive terminal interface for non-Docker environments.
//! Features include real-time progress monitoring, queue management, performance metrics,
//! and interactive controls for download operations.
//!
//! # Architecture
//!
//! The terminal UI is built with a modular architecture:
//! - `app`: Main application state machine and coordinator
//! - `layout`: Responsive layout system for different terminal sizes
//! - `events`: Event handling for keyboard, mouse, and terminal resize
//! - `renderer`: Efficient rendering engine with minimal redraws
//! - `state`: Application state management and synchronization
//! - `panels`: Individual UI panels (header, queue, stats, logs, footer)
//! - `widgets`: Reusable UI components (progress bars, charts, tables)
//!
//! # Usage
//!
//! The terminal UI is automatically enabled for non-Docker environments when using
//! compatible commands like `download`. It can be explicitly controlled with:
//! - `--ui terminal`: Force terminal UI mode
//! - `--ui simple`: Force simple text UI mode

pub mod app;
pub mod layout;
pub mod events;
pub mod renderer;
pub mod state;
pub mod themes;
pub mod colors;
pub mod styles;
pub mod widgets;
pub mod panels;
pub mod focus;
pub mod app_with_focus;
pub mod refresh;
pub mod state_buffer;
pub mod input;
pub mod integration;

#[cfg(test)]
pub mod integration_tests;

// Re-export main types for convenience
pub use app::{TerminalApp, AppMode};
pub use layout::{LayoutMode, LayoutManager};
pub use events::{EventHandler, AppEvent};
pub use state::{AppState, UpdateMessage};
pub use themes::{Theme, ThemeManager};
pub use colors::{ColorPalette, ColorRole};
pub use styles::ComponentStyles;
pub use renderer::{Renderer, RenderConfig, RenderPerformance};
pub use focus::{FocusManager, Focusable, FocusDirection};
pub use refresh::{RefreshManager, RefreshConfig, RefreshEvent, RefreshTrigger, EventLoop};
pub use state_buffer::{StateBuffer, BufferConfig, StateSnapshot};
pub use input::{InputAction, InputContext, QueueInputHandler, TrackAction, ScrollDirection};
pub use panels::{QueuePanel, HeaderPanel, FooterPanel, MainPanel, SidebarPanel};
pub use integration::{DownloadIntegration, QueueAction, DownloadEvent, DownloadSystemBridge};

/// Check if terminal UI should be enabled based on environment
pub fn should_enable_terminal_ui() -> bool {
    // Only enable in non-Docker environments
    if std::env::var("DOCKER").is_ok() {
        return false;
    }

    // Check if we have a proper terminal
    if !atty::is(atty::Stream::Stdout) {
        return false;
    }

    // Check terminal capabilities
    if let Ok((width, height)) = crossterm::terminal::size() {
        // Minimum size requirements for terminal UI
        width >= 80 && height >= 24
    } else {
        false
    }
}

/// Initialize terminal for TUI mode
pub fn init_terminal() -> Result<ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>, Box<dyn std::error::Error>> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
    let terminal = ratatui::Terminal::new(backend)?;

    Ok(terminal)
}

/// Restore terminal to normal mode
pub fn restore_terminal() -> Result<(), Box<dyn std::error::Error>> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_enable_terminal_ui_docker() {
        std::env::set_var("DOCKER", "1");
        assert!(!should_enable_terminal_ui());
        std::env::remove_var("DOCKER");
    }
}