use std::process::Command;
use std::thread;
use std::time::Duration;
use anyhow::{anyhow, Context, Result};

pub struct PasteHandler;

impl PasteHandler {
    pub fn get_active_window_id() -> Result<String> {
        let output = Command::new("xdotool")
            .arg("getactivewindow")
            .output()
            .context("Failed to get active window ID")?;

        if !output.status.success() {
            return Err(anyhow!("xdotool command failed"));
        }

        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }

    pub fn paste(window_id: &str) -> Result<()> {
        // 1. Restore focus immediately
        let _ = Command::new("xdotool")
            .arg("windowactivate")
            .arg(window_id)
            .status()?;

        // 2. Wait 300ms for focus to settle (More robust for Linux)
        thread::sleep(Duration::from_millis(300));

        // 3. Inject Ctrl+V with --clearmodifiers to ignore the Super key
        let _ = Command::new("xdotool")
            .arg("key")
            .arg("--clearmodifiers")
            .arg("ctrl+v")
            .status()
            .context("Failed to inject paste command")?;

        Ok(())
    }
}
