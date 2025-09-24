use async_trait::async_trait;
use crossterm::{
    cursor,
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self},
};
use std::io::{self, Write};
use std::time::Instant;

use crate::ui::{ProgressInterface, format_duration, calculate_eta};
use crate::ui::progress_state::{ProgressState, TrackResult, FinalStats};

/// Interactive TUI for terminal environments
pub struct TerminalUi {
    start_time: Option<Instant>,
    last_update: Option<Instant>,
    is_initialized: bool,
    width: usize,
}

const DEFAULT_WIDTH: usize = 63;

impl TerminalUi {
    pub fn new() -> Self {
        let width = if let Ok((w, _)) = terminal::size() {
            std::cmp::max(50, std::cmp::min(w as usize, 80))
        } else {
            DEFAULT_WIDTH
        };

        Self {
            start_time: None,
            last_update: None,
            is_initialized: false,
            width,
        }
    }

    fn get_border_line(&self) -> String {
        "─".repeat(self.width - 2)
    }

    fn visual_width(text: &str) -> usize {
        use unicode_width::UnicodeWidthStr;
        text.width()
    }

    fn pad_line(&self, content: &str) -> String {
        let content_visual_width = Self::visual_width(content);
        let available_space = self.width.saturating_sub(4); // -4 for "│ " and " │"

        let padding = if content_visual_width < available_space {
            available_space - content_visual_width
        } else {
            0
        };

        format!("│ {}{} │", content, " ".repeat(padding))
    }

    fn draw_border_line(&self, border_type: &str) -> io::Result<()> {
        execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }))?;
        match border_type {
            "top" => execute!(io::stdout(), Print(format!("┌{}┐", self.get_border_line())))?,
            "middle" => execute!(io::stdout(), Print(format!("├{}┤", self.get_border_line())))?,
            "bottom" => execute!(io::stdout(), Print(format!("└{}┘", self.get_border_line())))?,
            _ => {}
        }
        execute!(io::stdout(), ResetColor)?;
        Ok(())
    }

    fn draw_content_line(&self, content: &str) -> io::Result<()> {
        let content_visual_width = Self::visual_width(content);
        let available_space = self.width.saturating_sub(4); // -4 for "│ " and " │"
        let padding = if content_visual_width < available_space {
            available_space - content_visual_width
        } else {
            0
        };

        execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }))?;
        execute!(io::stdout(), Print("│"))?;
        execute!(io::stdout(), Print(" "))?;
        execute!(io::stdout(), Print(content))?;
        execute!(io::stdout(), Print(&" ".repeat(padding)))?;
        execute!(io::stdout(), Print(" │"))?;
        execute!(io::stdout(), ResetColor)?;
        Ok(())
    }

    fn init_terminal(&mut self) -> io::Result<()> {
        if !self.is_initialized {
            execute!(io::stdout(), terminal::Clear(terminal::ClearType::All))?;
            execute!(io::stdout(), cursor::Hide)?;
            self.is_initialized = true;
        }
        Ok(())
    }

    fn cleanup_terminal(&self) -> io::Result<()> {
        execute!(io::stdout(), cursor::Show, ResetColor)?;
        Ok(())
    }

    fn draw_header(&self) -> io::Result<()> {
        execute!(io::stdout(), cursor::MoveTo(0, 0))?;
        self.draw_border_line("top")?;
        execute!(io::stdout(), cursor::MoveToNextLine(1))?;
        self.draw_content_line(&format!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")))?;
        execute!(io::stdout(), cursor::MoveToNextLine(1))?;
        self.draw_border_line("middle")?;
        Ok(())
    }

    fn draw_progress_bar(&self, percentage: f64) -> io::Result<()> {
        execute!(io::stdout(), cursor::MoveTo(0, 3))?;

        let percentage_str = format!(" {:.1}% ", percentage); // Add spaces around percentage
        let percentage_width = Self::visual_width(&percentage_str);

        // Calculate bar width to fill ALL available space, respecting padding
        let total_available = self.width.saturating_sub(4); // -4 for "│ " and " │"
        let bar_width = total_available; // Use all available space

        // Calculate filled portion (where to place the percentage)
        let filled_chars = ((percentage / 100.0) * bar_width as f64) as usize;

        // Draw left border
        execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }))?;
        execute!(io::stdout(), Print("│"))?;
        execute!(io::stdout(), Print(" "))?; // Left padding

        // Draw progress bar with percentage inside
        execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }))?;

        if percentage == 0.0 {
            // Special case: 0% - show percentage at the beginning
            execute!(io::stdout(), Print(&percentage_str))?;
            execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
            let remaining = bar_width.saturating_sub(percentage_width);
            execute!(io::stdout(), Print("░".repeat(remaining)))?;
        } else if percentage >= 100.0 {
            // Special case: 100% - show percentage at the end
            let before_percentage = bar_width.saturating_sub(percentage_width);
            execute!(io::stdout(), Print("█".repeat(before_percentage)))?;
            execute!(io::stdout(), Print(&percentage_str))?;
        } else {
            // Normal case: percentage inside the filled portion
            // Ensure we don't exceed bar_width by being more careful with calculations
            let total_filled = filled_chars.min(bar_width);
            let percentage_pos = if total_filled >= percentage_width {
                total_filled - percentage_width
            } else {
                0
            };

            // Draw filled part before percentage
            execute!(io::stdout(), Print("█".repeat(percentage_pos)))?;

            // Draw percentage
            execute!(io::stdout(), Print(&percentage_str))?;

            // Calculate remaining space carefully
            let used_chars = percentage_pos + percentage_width;
            let remaining_filled = if used_chars < total_filled {
                total_filled - used_chars
            } else {
                0
            };

            // Draw remaining filled part after percentage
            execute!(io::stdout(), Print("█".repeat(remaining_filled)))?;

            // Draw empty part - ensure total doesn't exceed bar_width
            let total_used = percentage_pos + percentage_width + remaining_filled;
            let empty_chars = bar_width.saturating_sub(total_used);
            execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
            execute!(io::stdout(), Print("░".repeat(empty_chars)))?;
        }

        // Draw right border
        execute!(io::stdout(), Print(" "))?; // Right padding
        execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }))?;
        execute!(io::stdout(), Print("│"))?; // Right border
        execute!(io::stdout(), ResetColor)?;

        Ok(())
    }

    fn draw_statistics(&self, state: &ProgressState) -> io::Result<()> {
        execute!(io::stdout(), cursor::MoveTo(0, 4))?;

        // Single line with all statistics - no title, no border
        let stats_line = format!("Total: {} - Synced: {} - Plain: {} - Missing: {}",
            state.total_tracks, state.synced_tracks, state.plain_tracks, state.missing_tracks);
        self.draw_content_line(&stats_line)?;

        Ok(())
    }

    fn draw_current_operation(&self, state: &ProgressState) -> io::Result<()> {
        execute!(io::stdout(), cursor::MoveToNextLine(1))?;
        self.draw_border_line("middle")?;
        execute!(io::stdout(), cursor::MoveToNextLine(1))?;

        self.draw_content_line("Current Operation:")?;
        execute!(io::stdout(), cursor::MoveToNextLine(1))?;

        // Operation text (truncate if too long)
        let max_visual_width = self.width.saturating_sub(7); // Leave space for borders, padding, and "..."
        let operation = if Self::visual_width(&state.current_operation) > max_visual_width {
            // Truncate by characters until we fit within the visual width
            let mut truncated = String::new();
            let mut current_width = 0;
            for ch in state.current_operation.chars() {
                let ch_width = Self::visual_width(&ch.to_string());
                if current_width + ch_width + 3 > max_visual_width { // +3 for "..."
                    break;
                }
                truncated.push(ch);
                current_width += ch_width;
            }
            format!("{}...", truncated)
        } else {
            state.current_operation.clone()
        };

        self.draw_content_line(&operation)?;

        Ok(())
    }

    fn draw_footer(&self, state: &ProgressState, controls_text: Option<&str>) -> io::Result<()> {
        execute!(io::stdout(), cursor::MoveToNextLine(1))?;
        self.draw_border_line("middle")?;
        execute!(io::stdout(), cursor::MoveToNextLine(1))?;

        let speed = state.tracks_per_second();
        let elapsed = format_duration(state.start_time.elapsed());

        let eta_str = if let Some(eta) = calculate_eta(state.start_time, state.processed_tracks, state.total_tracks) {
            format_duration(eta)
        } else {
            "--:--".to_string()
        };

        let footer_content = format!(
            "Speed: {:.1} tracks/sec | ETA: {} | Elapsed: {}",
            speed, eta_str, elapsed
        );

        self.draw_content_line(&footer_content)?;

        // Add keyboard controls section
        if let Some(controls) = controls_text {
            execute!(io::stdout(), cursor::MoveToNextLine(1))?;
            self.draw_border_line("middle")?;
            execute!(io::stdout(), cursor::MoveToNextLine(1))?;
            self.draw_content_line(controls)?;
        }

        execute!(io::stdout(), cursor::MoveToNextLine(1))?;
        self.draw_border_line("bottom")?;

        Ok(())
    }

    fn should_update(&mut self) -> bool {
        let now = Instant::now();
        if let Some(last) = self.last_update {
            if now.duration_since(last).as_millis() < 100 {  // Throttle updates to 10 FPS
                return false;
            }
        }
        self.last_update = Some(now);
        true
    }
}

impl Drop for TerminalUi {
    fn drop(&mut self) {
        if self.is_initialized {
            let _ = self.cleanup_terminal();
        }
    }
}

#[async_trait]
impl ProgressInterface for TerminalUi {
    async fn start(&mut self, total: usize) {
        self.start_time = Some(Instant::now());
        let _ = self.init_terminal();

        let state = ProgressState::new(total);
        let _ = self.draw_header();
        let _ = self.draw_progress_bar(0.0);
        let _ = self.draw_statistics(&state);
        let _ = self.draw_current_operation(&state);
        let _ = self.draw_footer(&state, None);
        let _ = io::stdout().flush();
    }

    async fn update_progress(&mut self, state: &ProgressState) {
        self.update_progress_with_controls(state, None).await;
    }

    async fn update_progress_with_controls(&mut self, state: &ProgressState, controls_text: Option<&str>) {
        if !self.should_update() {
            return;
        }

        let _ = self.draw_progress_bar(state.progress_percentage());
        let _ = self.draw_statistics(state);
        let _ = self.draw_current_operation(state);
        let _ = self.draw_footer(state, controls_text);
        let _ = io::stdout().flush();
    }

    async fn set_operation(&mut self, operation: String) {
        // Operation updates will be handled by update_progress
        let _ = operation; // Silence unused variable warning
    }

    async fn track_completed(&mut self, _track: &str, _result: TrackResult) {
        // Track completion will be reflected in the next progress update
    }

    async fn handle_error(&mut self, _track: &str, _error: &str) {
        // Errors will be reflected in statistics
    }

    async fn finish(&mut self, final_stats: &FinalStats) {
        let _ = self.cleanup_terminal();

        // Show summary below the layout (no title, placed right after layout)
        println!();

        // Summary statistics without titles, concise format
        if final_stats.synced_tracks > 0 {
            println!("Synced Lyrics: {} tracks", final_stats.synced_tracks);
        }
        if final_stats.plain_tracks > 0 {
            println!("Plain Lyrics: {} tracks", final_stats.plain_tracks);
        }
        if final_stats.missing_tracks > 0 {
            println!("Not Found: {} tracks", final_stats.missing_tracks);
        }
        if final_stats.error_tracks > 0 {
            println!("Errors: {} tracks", final_stats.error_tracks);
        }

        // Performance metrics
        println!("Success Rate: {:.1}%", final_stats.success_rate);
        println!("Speed: {:.1} tracks/sec", final_stats.tracks_per_second);
        println!("Duration: {}", format_duration(final_stats.total_duration));

        // Final message
        let successful = final_stats.successful_tracks();
        let failed = final_stats.failed_tracks();

        if failed == 0 {
            println!("\nAll {} tracks processed successfully!", successful);
        } else {
            println!("\n{} successful, {} failed out of {} total tracks",
                successful, failed, final_stats.total_tracks);
        }
    }
}

impl TerminalUi {
    fn print_summary_line(&self, content: &str, width: usize) {
        // Force the calculation to ensure correct alignment
        let total_line_width = width;
        let border_width = 4; // "│ " + " │"
        let content_space = total_line_width - border_width;
        let content_visual_width = Self::visual_width(content);

        let padding = if content_visual_width < content_space {
            content_space - content_visual_width
        } else {
            0
        };

        // Build the entire line as a string first to ensure exact width
        let full_line = format!("│ {}{} │", content, " ".repeat(padding));
        let actual_width = Self::visual_width(&full_line);

        // If there's still a mismatch, adjust padding
        let adjusted_padding = if actual_width != total_line_width {
            if actual_width < total_line_width {
                padding + (total_line_width - actual_width)
            } else {
                padding.saturating_sub(actual_width - total_line_width)
            }
        } else {
            padding
        };

        execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 })).unwrap();
        print!("│");
        execute!(io::stdout(), ResetColor).unwrap();
        print!(" {}{} ", content, " ".repeat(adjusted_padding));
        execute!(io::stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 })).unwrap();
        println!("│");
        execute!(io::stdout(), ResetColor).unwrap();
    }
}