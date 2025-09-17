use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use tokio::signal;
use tokio::sync::mpsc;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{self, disable_raw_mode, enable_raw_mode},
};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppState {
    Running = 0,
    Paused = 1,
    Stopping = 2,
    Stopped = 3,
}

impl From<u8> for AppState {
    fn from(value: u8) -> Self {
        match value {
            0 => AppState::Running,
            1 => AppState::Paused,
            2 => AppState::Stopping,
            3 => AppState::Stopped,
            _ => AppState::Running,
        }
    }
}

#[derive(Clone)]
pub struct SignalHandler {
    state: Arc<AtomicU8>,
    shutdown_requested: Arc<AtomicBool>,
    last_esc_time: std::time::Instant,
}

impl SignalHandler {
    pub fn new() -> Self {
        Self {
            state: Arc::new(AtomicU8::new(AppState::Running as u8)),
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            last_esc_time: std::time::Instant::now(),
        }
    }

    pub fn get_state(&self) -> AppState {
        AppState::from(self.state.load(Ordering::Acquire))
    }

    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::Acquire)
    }

    pub fn pause(&self) {
        self.state.store(AppState::Paused as u8, Ordering::Release);
        info!("Downloads paused by user");
    }

    pub fn resume(&self) {
        if self.get_state() == AppState::Paused {
            self.state.store(AppState::Running as u8, Ordering::Release);
            info!("Downloads resumed by user");
        }
    }

    pub fn request_shutdown(&self) {
        self.state.store(AppState::Stopping as u8, Ordering::Release);
        self.shutdown_requested.store(true, Ordering::Release);
        info!("Graceful shutdown requested");
    }

    pub fn mark_stopped(&self) {
        self.state.store(AppState::Stopped as u8, Ordering::Release);
    }

    pub fn shutdown(&self) {
        self.shutdown_requested.store(true, Ordering::Release);
        self.state.store(AppState::Stopped as u8, Ordering::Release);
    }

    pub fn enable_input_monitoring(&self) -> Result<(), std::io::Error> {
        enable_raw_mode()?;
        Ok(())
    }

    pub fn disable_input_monitoring(&self) -> Result<(), std::io::Error> {
        disable_raw_mode()?;
        Ok(())
    }

    pub async fn start_signal_monitoring(&mut self) -> (mpsc::Receiver<AppState>, Vec<tokio::task::JoinHandle<()>>) {
        let (tx, rx) = mpsc::channel(32);
        let mut handles = Vec::new();

        // Handle SIGTERM
        let state_clone = Arc::clone(&self.state);
        let shutdown_clone = Arc::clone(&self.shutdown_requested);
        let tx_signal = tx.clone();

        let sigterm_handle = tokio::spawn(async move {
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to create SIGTERM handler");

            loop {
                tokio::select! {
                    _ = sigterm.recv() => {
                        warn!("Received SIGTERM signal");
                        state_clone.store(AppState::Stopping as u8, Ordering::Release);
                        shutdown_clone.store(true, Ordering::Release);
                        let _ = tx_signal.send(AppState::Stopping).await;
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                        // Check if we should stop monitoring
                        if shutdown_clone.load(Ordering::Acquire) {
                            break;
                        }
                    }
                }
            }
        });
        handles.push(sigterm_handle);

        // Handle keyboard input
        let tx_keyboard = tx.clone();
        let state_clone = Arc::clone(&self.state);
        let shutdown_clone = Arc::clone(&self.shutdown_requested);

        let keyboard_handle = tokio::spawn(async move {
            let mut last_esc_time = std::time::Instant::now();

            // Enable raw mode for input capture
            if let Err(e) = enable_raw_mode() {
                warn!("Failed to enable raw mode: {}", e);
                return;
            }

            loop {
                // Check if we should stop monitoring
                if shutdown_clone.load(Ordering::Acquire) {
                    break;
                }

                // Use a longer timeout to reduce CPU usage
                if event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
                    match event::read() {
                        Ok(Event::Key(key)) => {
                            // Only process key presses, not releases
                            if key.kind == crossterm::event::KeyEventKind::Press {
                                match key {
                                    KeyEvent { code: KeyCode::Esc, modifiers: KeyModifiers::NONE, .. } => {
                                        let now = std::time::Instant::now();
                                        if now.duration_since(last_esc_time) < std::time::Duration::from_millis(500) {
                                            // Double ESC detected
                                            info!("Double ESC detected - requesting graceful shutdown");
                                            state_clone.store(AppState::Stopping as u8, Ordering::Release);
                                            shutdown_clone.store(true, Ordering::Release);
                                            let _ = tx_keyboard.send(AppState::Stopping).await;
                                            break;
                                        }
                                        last_esc_time = now;
                                    }
                                    KeyEvent { code: KeyCode::Char('p'), modifiers: KeyModifiers::NONE, .. } |
                                    KeyEvent { code: KeyCode::Char('P'), modifiers: KeyModifiers::NONE, .. } => {
                                        let current_state = AppState::from(state_clone.load(Ordering::Acquire));
                                        if current_state == AppState::Running {
                                            info!("Downloads paused by user (P key)");
                                            state_clone.store(AppState::Paused as u8, Ordering::Release);
                                            let _ = tx_keyboard.send(AppState::Paused).await;
                                        }
                                    }
                                    KeyEvent { code: KeyCode::Char('r'), modifiers: KeyModifiers::NONE, .. } |
                                    KeyEvent { code: KeyCode::Char('R'), modifiers: KeyModifiers::NONE, .. } => {
                                        let current_state = AppState::from(state_clone.load(Ordering::Acquire));
                                        if current_state == AppState::Paused {
                                            info!("Downloads resumed by user (R key)");
                                            state_clone.store(AppState::Running as u8, Ordering::Release);
                                            let _ = tx_keyboard.send(AppState::Running).await;
                                        }
                                    }
                                    KeyEvent { code: KeyCode::Char('q'), modifiers: KeyModifiers::CONTROL, .. } => {
                                        // Ctrl+Q as backup exit method
                                        info!("Ctrl+Q detected - requesting graceful shutdown");
                                        state_clone.store(AppState::Stopping as u8, Ordering::Release);
                                        shutdown_clone.store(true, Ordering::Release);
                                        let _ = tx_keyboard.send(AppState::Stopping).await;
                                        break;
                                    }
                                    _ => {
                                        // Ignore other keys silently
                                    }
                                }
                            }
                        }
                        Ok(_) => {
                            // Ignore other events (mouse, resize, etc.)
                        }
                        Err(_) => {
                            // Error reading input, continue
                            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                        }
                    }
                }
            }

            // Disable raw mode when exiting
            let _ = disable_raw_mode();
        });
        handles.push(keyboard_handle);

        (rx, handles)
    }

    pub fn get_status_text(&self) -> String {
        match self.get_state() {
            AppState::Running => "Press P to pause, Double ESC to quit".to_string(),
            AppState::Paused => "PAUSED - Press R to resume, Double ESC to quit".to_string(),
            AppState::Stopping => "Finishing current downloads...".to_string(),
            AppState::Stopped => "Stopped".to_string(),
        }
    }
}

impl Default for SignalHandler {
    fn default() -> Self {
        Self::new()
    }
}