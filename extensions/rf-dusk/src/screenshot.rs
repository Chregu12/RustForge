//! Screenshot functionality

use crate::DuskResult;
use std::path::PathBuf;

/// Screenshot configuration
#[derive(Debug, Clone)]
pub struct ScreenshotConfig {
    pub directory: PathBuf,
    pub format: ScreenshotFormat,
    pub full_page: bool,
}

impl Default for ScreenshotConfig {
    fn default() -> Self {
        Self {
            directory: PathBuf::from("tests/Browser/screenshots"),
            format: ScreenshotFormat::Png,
            full_page: false,
        }
    }
}

/// Screenshot format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenshotFormat {
    Png,
    Jpeg,
}

/// Screenshot helper
pub struct Screenshot {
    pub data: Vec<u8>,
    pub name: String,
    pub path: PathBuf,
}

impl Screenshot {
    /// Create a new screenshot
    pub fn new(data: Vec<u8>, name: impl Into<String>, path: PathBuf) -> Self {
        Self {
            data,
            name: name.into(),
            path,
        }
    }

    /// Save screenshot to file
    pub fn save(&self) -> DuskResult<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, &self.data)?;
        Ok(())
    }

    /// Compare with a baseline screenshot
    pub fn compare_with_baseline(&self, baseline_path: &PathBuf) -> DuskResult<ScreenshotComparison> {
        if !baseline_path.exists() {
            return Ok(ScreenshotComparison::NoBaseline);
        }

        let baseline_data = std::fs::read(baseline_path)?;

        if self.data == baseline_data {
            return Ok(ScreenshotComparison::Match);
        }

        // For more sophisticated comparison, you'd use image processing
        // This is a simple byte comparison
        Ok(ScreenshotComparison::Different {
            difference_percent: calculate_difference(&self.data, &baseline_data),
        })
    }

    /// Update the baseline with this screenshot
    pub fn update_baseline(&self, baseline_path: &PathBuf) -> DuskResult<()> {
        if let Some(parent) = baseline_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(baseline_path, &self.data)?;
        Ok(())
    }
}

/// Screenshot comparison result
#[derive(Debug)]
pub enum ScreenshotComparison {
    /// Screenshots match
    Match,
    /// Screenshots are different
    Different { difference_percent: f64 },
    /// No baseline exists
    NoBaseline,
}

impl ScreenshotComparison {
    /// Check if screenshots match within tolerance
    pub fn matches_within(&self, tolerance: f64) -> bool {
        match self {
            ScreenshotComparison::Match => true,
            ScreenshotComparison::Different { difference_percent } => *difference_percent <= tolerance,
            ScreenshotComparison::NoBaseline => false,
        }
    }
}

/// Calculate difference percentage between two images
fn calculate_difference(a: &[u8], b: &[u8]) -> f64 {
    if a.len() != b.len() {
        return 100.0;
    }

    let different_bytes = a.iter().zip(b.iter()).filter(|(x, y)| x != y).count();
    (different_bytes as f64 / a.len() as f64) * 100.0
}

/// Screenshot assertion macros
#[macro_export]
macro_rules! assert_screenshot {
    ($browser:expr, $name:expr) => {{
        let screenshot = $browser.screenshot($name).await?;
        let screenshot = $crate::Screenshot::new(
            screenshot,
            $name,
            std::path::PathBuf::from(format!("tests/Browser/screenshots/{}.png", $name)),
        );

        let baseline = std::path::PathBuf::from(format!("tests/Browser/baselines/{}.png", $name));
        let comparison = screenshot.compare_with_baseline(&baseline)?;

        match comparison {
            $crate::screenshot::ScreenshotComparison::NoBaseline => {
                screenshot.update_baseline(&baseline)?;
                tracing::warn!("Created baseline screenshot for {}", $name);
            }
            $crate::screenshot::ScreenshotComparison::Different { difference_percent } => {
                screenshot.save()?;
                return Err($crate::DuskError::AssertionFailed(format!(
                    "Screenshot '{}' differs from baseline by {:.2}%",
                    $name, difference_percent
                )));
            }
            $crate::screenshot::ScreenshotComparison::Match => {}
        }
    }};
}
