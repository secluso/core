//! SPDX-License-Identifier: GPL-3.0-or-later
use crate::pi_hub_provision::events::handle_event_line;
use anyhow::{anyhow, bail, Context, Result};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use tauri::AppHandle;
use uuid::Uuid;

const DOCKERFILE_TXT: &str = include_str!("../../assets/pi_hub/Dockerfile");
const BUILD_SH_TXT: &str = include_str!("../../assets/pi_hub/build_image.sh");

pub fn write_docker_context(dir: &Path) -> Result<()> {
  // embed the build assets so we can run without shipping extra files
  fs::write(dir.join("Dockerfile"), DOCKERFILE_TXT).context("write Dockerfile")?;
  fs::write(dir.join("build.sh"), BUILD_SH_TXT).context("write build.sh")?;
  Ok(())
}

pub fn docker_version() -> Result<String> {
  // basic docker health check used for the status page
  let out = Command::new("docker")
    .args(["--version"])
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()
    .map_err(|_| anyhow!("Docker is not installed or not available on PATH. Install Docker and try again."))?;

  if !out.status.success() {
    bail!("Docker is not installed or not available on PATH. Install Docker and try again.");
  }

  Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn docker_ready() -> Result<String> {
  let client_version = docker_version()?;
  let out = Command::new("docker")
    .args(["info", "--format", "{{.ServerVersion}}"])
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()
    .map_err(|_| anyhow!("Docker is installed, but the Docker daemon is not reachable. Start Docker and try again."))?;

  if !out.status.success() {
    bail!("Docker is installed, but the Docker daemon is not reachable. Start Docker and try again.");
  }

  let server_version = String::from_utf8_lossy(&out.stdout).trim().trim_matches('"').to_string();
  if server_version.is_empty() {
    return Ok(client_version);
  }

  Ok(format!("{client_version} | server {server_version}"))
}

pub fn run_with_output(app: &AppHandle, run_id: Uuid, step: &str, cmd: &mut Command) -> Result<()> {
  // stream stdout and stderr into ui events
  handle_event_line(app, run_id, "info", step, &format!("running: {:?}", cmd));
  let mut child = cmd
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .with_context(|| format!("failed to spawn {:?}", cmd))?;

  let stdout = child.stdout.take().unwrap();
  let stderr = child.stderr.take().unwrap();

  let step_out = step.to_string();
  let app_out = app.clone();
  let run_out = run_id;
  let out_handle = thread::spawn(move || {
    let reader = BufReader::new(stdout);
    for line in reader.lines().flatten() {
      handle_event_line(&app_out, run_out, "info", &step_out, &line);
    }
  });

  let step_err = step.to_string();
  let app_err = app.clone();
  let run_err = run_id;
  let err_handle = thread::spawn(move || {
    let reader = BufReader::new(stderr);
    for line in reader.lines().flatten() {
      handle_event_line(&app_err, run_err, "error", &step_err, &line);
    }
  });

  let status = child.wait()?;
  let _ = out_handle.join();
  let _ = err_handle.join();

  if !status.success() {
    bail!("command failed with status: {} ({:?})", status, cmd);
  }

  Ok(())
}

pub fn err_to_string(e: anyhow::Error) -> String {
  format!("{:#}", e)
}

pub struct DockerCleanup {
  volume: String,
  container: String,
}

impl DockerCleanup {
  pub fn new(volume: String, container: String) -> Self {
    Self { volume, container }
  }
}

impl Drop for DockerCleanup {
  fn drop(&mut self) {
    let _ = Command::new("docker")
      .args(["rm", "-f", &self.container])
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .status();
    let _ = Command::new("docker")
      .args(["volume", "rm", "-f", &self.volume])
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .status();
  }
}
