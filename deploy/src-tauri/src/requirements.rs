//! SPDX-License-Identifier: GPL-3.0-or-later

use serde::Serialize;
use std::process::Command;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementStatus {
  pub name: String,
  pub ok: bool,
  pub version: Option<String>,
  pub hint: String,
}

fn check_cmd(cmd: &str, args: &[&str]) -> (bool, Option<String>) {
  let out = Command::new(cmd).args(args).output();
  match out {
    Ok(res) if res.status.success() => {
      let stdout = String::from_utf8_lossy(&res.stdout).trim().to_string();
      let stderr = String::from_utf8_lossy(&res.stderr).trim().to_string();
      let version = if !stdout.is_empty() { stdout } else if !stderr.is_empty() { stderr } else { String::new() };
      let version = if version.is_empty() { None } else { Some(version) };
      (true, version)
    }
    _ => (false, None),
  }
}

#[tauri::command]
pub async fn check_requirements() -> Result<Vec<RequirementStatus>, String> {
  tauri::async_runtime::spawn_blocking(|| {
    let mut statuses = Vec::new();

    let checks = vec![
      ("Docker", "docker", vec!["--version"], "Needed to build Raspberry Pi images."),
      ("Docker Buildx", "docker", vec!["buildx", "version"], "Needed for reproducible release builds."),
      ("Git", "git", vec!["--version"], "Needed to fetch sources."),
      ("Node.js (18+)", "node", vec!["--version"], "Needed for UI dev."),
      ("pnpm", "pnpm", vec!["--version"], "Needed for UI dev."),
      ("Rust (1.85)", "rustc", vec!["--version"], "Needed for Tauri backend builds."),
      ("Cargo", "cargo", vec!["--version"], "Needed for Rust builds."),
    ];

    for (name, cmd, args, hint) in checks {
      let (ok, version) = check_cmd(cmd, &args);
      statuses.push(RequirementStatus {
        name: name.to_string(),
        ok,
        version,
        hint: hint.to_string(),
      });
    }

    Ok(statuses)
  })
  .await
  .map_err(|e| e.to_string())?
  .map_err(|e: std::io::Error| e.to_string())
}
