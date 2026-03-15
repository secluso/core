//! SPDX-License-Identifier: GPL-3.0-or-later
mod build;
pub(crate) mod credentials;
mod docker;
mod events;
pub(crate) mod model;
pub(crate) mod temp;

use crate::pi_hub_provision::build::run_build_image;
use crate::pi_hub_provision::credentials::generate_user_credentials_only;
use crate::pi_hub_provision::docker::{docker_ready, err_to_string};
use crate::pi_hub_provision::events::{emit, log_line, ProvisionEvent};
use crate::pi_hub_provision::model::{SigKey, Wifi};
use crate::pi_hub_provision::temp::shared_temp_dir;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::AppHandle;
use uuid::Uuid;

// api wiring for tauri commands

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildImageRequest {
  variant: Option<String>,
  qr_output_path: String,
  image_output_path: String,
  ssh_enabled: Option<bool>,
  wifi: Option<Wifi>,
  binaries_repo: Option<String>,
  sig_keys: Option<Vec<SigKey>>,
  github_token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildImageResponse {
  out_image: String,
}

#[derive(Debug, Serialize)]
pub struct BuildStart {
  pub run_id: Uuid,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerStatus {
  ok: bool,
  version: Option<String>,
  message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateUserCredentialsRequest {
  server_url: String,
  output_path: String,
  qr_output_path: Option<String>,
}

#[tauri::command]
pub async fn generate_user_credentials(
  app: AppHandle,
  req: GenerateUserCredentialsRequest,
) -> std::result::Result<(), String> {
  tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<()> {
    let run_id = Uuid::new_v4();
    let work_dir = shared_temp_dir("secluso-user-creds").context("creating temp work dir")?;
    let work_path = work_dir.path();

    generate_user_credentials_only(&app, run_id, work_path, &req.server_url, "secluso/secluso", None, None)?;

    let out_path = Path::new(&req.output_path);
    if let Some(parent) = out_path.parent() {
      if !parent.as_os_str().is_empty() {
        fs::create_dir_all(parent)?;
      }
    }
    fs::copy(work_path.join("user_credentials"), &req.output_path)
      .with_context(|| format!("copying user_credentials to {}", req.output_path))?;

    if let Some(qr_out) = &req.qr_output_path {
      let qr_src = work_path.join("user_credentials_qrcode.png");
      if qr_src.exists() {
        let qr_path = Path::new(qr_out);
        if let Some(parent) = qr_path.parent() {
          if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
          }
        }
        fs::copy(&qr_src, qr_out).with_context(|| format!("copying QR code to {}", qr_out))?;
      } else {
        anyhow::bail!("Expected QR code missing at {}", qr_src.display());
      }
    }

    Ok(())
  })
  .await
  .map_err(|e| e.to_string())?
  .map_err(err_to_string)
}

#[tauri::command]
pub async fn check_docker() -> std::result::Result<DockerStatus, String> {
  tauri::async_runtime::spawn_blocking(|| -> anyhow::Result<DockerStatus> {
    match docker_ready() {
      Ok(version) => Ok(DockerStatus {
        ok: true,
        version: Some(version),
        message: None,
      }),
      Err(e) => Ok(DockerStatus {
        ok: false,
        version: None,
        message: Some(err_to_string(e)),
      }),
    }
  })
  .await
  .map_err(|e| e.to_string())?
  .map_err(err_to_string)
}

#[tauri::command]
pub async fn build_image(app: AppHandle, req: BuildImageRequest) -> std::result::Result<BuildStart, String> {
  let run_id = Uuid::new_v4();
  let app2 = app.clone();

  tokio::task::spawn_blocking(move || {
    match run_build_image(&app2, run_id, req) {
      Ok(result) => {
        log_line(&app2, run_id, "info", Some("result"), format!("Image saved at: {}", result.out_image));
        emit(&app2, ProvisionEvent::Done { run_id, ok: true });
      }
      Err(e) => {
        log_line(&app2, run_id, "error", Some("fatal"), format!("{e:#}"));
        emit(&app2, ProvisionEvent::Done { run_id, ok: false });
      }
    }
  });

  Ok(BuildStart { run_id })
}
