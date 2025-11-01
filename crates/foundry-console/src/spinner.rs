use std::io::{self, Write};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub enum SpinnerStyle {
    Dots,
    Line,
    Arc,
    Circle,
    BouncingBar,
}

impl SpinnerStyle {
    fn frames(&self) -> &[&str] {
        match self {
            SpinnerStyle::Dots => &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            SpinnerStyle::Line => &["-", "\\", "|", "/"],
            SpinnerStyle::Arc => &["◜", "◠", "◝", "◞", "◡", "◟"],
            SpinnerStyle::Circle => &["◐", "◓", "◑", "◒"],
            SpinnerStyle::BouncingBar => &[
                "[    ]",
                "[=   ]",
                "[==  ]",
                "[=== ]",
                "[ ===]",
                "[  ==]",
                "[   =]",
                "[    ]",
                "[   =]",
                "[  ==]",
                "[ ===]",
                "[=== ]",
                "[==  ]",
                "[=   ]",
            ],
        }
    }
}

pub struct Spinner {
    message: String,
    style: SpinnerStyle,
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Spinner {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            style: SpinnerStyle::Dots,
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }

    pub fn with_style(mut self, style: SpinnerStyle) -> Self {
        self.style = style;
        self
    }

    pub fn start(&mut self) {
        self.running.store(true, Ordering::SeqCst);
        let frames: Vec<String> = self.style.frames().iter().map(|s| s.to_string()).collect();
        let message = self.message.clone();
        let running = Arc::clone(&self.running);

        let handle = thread::spawn(move || {
            let mut idx = 0;
            while running.load(Ordering::SeqCst) {
                let frame = &frames[idx % frames.len()];
                print!("\r{} {}", frame, message);
                io::stdout().flush().unwrap_or(());

                thread::sleep(Duration::from_millis(80));
                idx += 1;
            }
            // Clear the line
            print!("\r{}\r", " ".repeat(message.len() + 10));
            io::stdout().flush().unwrap_or(());
        });

        self.handle = Some(handle);
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    pub fn stop_with_message(&mut self, message: &str) {
        self.stop();
        println!("{}", message);
    }

    pub fn update_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_creation() {
        let spinner = Spinner::new("Loading");
        assert_eq!(spinner.message, "Loading");
    }

    #[test]
    fn test_spinner_frames() {
        let frames = SpinnerStyle::Dots.frames();
        assert!(!frames.is_empty());
    }
}
