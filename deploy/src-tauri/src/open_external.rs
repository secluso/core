//! SPDX-License-Identifier: GPL-3.0-or-later

use std::process::Command;

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("Only http(s) URLs are allowed.".to_string());
    }

    let status = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "start", "", &url]).status()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(&url).status()
    } else {
        Command::new("xdg-open").arg(&url).status()
    };

    match status {
        Ok(res) if res.success() => Ok(()),
        Ok(res) => Err(format!(
            "Failed to open URL (exit code {}).",
            res.code().unwrap_or(-1)
        )),
        Err(err) => Err(format!("Failed to open URL: {err}")),
    }
}
