use std::process::Command;
use std::thread;
use std::time::Duration;
use anyhow::{anyhow, Context, Result};

pub struct PasteHandler;

impl PasteHandler {
    /// Returns the X11 window ID of the currently active window.
    /// Must be called *before* the clipboard UI opens so we capture the
    /// window the user was typing in, not our own window.
    pub fn get_active_window_id() -> Result<String> {
        let output = Command::new("xdotool")
            .arg("getactivewindow")
            .output()
            .context("Failed to get active window ID")?;

        if !output.status.success() {
            return Err(anyhow!("xdotool getactivewindow failed"));
        }

        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }

    /// Activate `window_id`, wait for focus to settle, then inject Ctrl+V.
    ///
    /// Called *after* the clipboard content has been set (with an 80 ms
    /// delay inside daemon.rs), so the paste always reads the correct entry.
    pub fn paste(window_id: &str) -> Result<()> {
        // 1. Raise and focus the target window
        Command::new("xdotool")
            .args(["windowactivate", "--sync", window_id])
            .status()
            .context("xdotool windowactivate failed")?;

        Command::new("xdotool")
            .args(["windowfocus", "--sync", window_id])
            .status()
            .context("xdotool windowfocus failed")?;

        // 2. Short wait for the WM to hand focus over
        thread::sleep(Duration::from_millis(100));

        // 3. Inject Ctrl+V; --clearmodifiers prevents Super/Shift bleed
        Command::new("xdotool")
            .args(["key", "--clearmodifiers", "ctrl+v"])
            .status()
            .context("xdotool key ctrl+v failed")?;

        Ok(())
    }

    /// Send a SET command to the running daemon over the Unix socket.
    /// The daemon sets the clipboard then pastes into `prev_window_id`.
    ///
    /// Protocol: "SET:<hash>:<window_id>"
    pub fn send_set_command(hash: &str, prev_window_id: Option<&str>) -> Result<()> {
        use std::io::Write;
        use std::os::unix::net::UnixStream;

        let mut stream = UnixStream::connect("/tmp/paste_it_daemon.sock")
            .context("Could not connect to daemon socket")?;

        let wid = prev_window_id.unwrap_or("");
        let msg = format!("SET:{}:{}", hash, wid);
        stream.write_all(msg.as_bytes())
            .context("Failed to write to daemon socket")?;

        Ok(())
    }
}