//! SPDX-License-Identifier: GPL-3.0-or-later
use crate::pi_hub_provision::model::SigKey;
use anyhow::{anyhow, Context, Result};
use image::Luma;
use openmls_rust_crypto::OpenMlsRustCrypto;
use openmls_traits::random::OpenMlsRand;
use openmls_traits::OpenMlsProvider;
use qrcode::QrCode;
use secluso_client_server_lib::auth::create_user_credentials;
use std::fs;
use std::path::Path;
use tauri::AppHandle;
use url::Url;
use uuid::Uuid;

const NUM_CAMERA_SECRET_BYTES: usize = 72;
const CAMERA_SECRET_VERSION: &str = "v1.1";

#[derive(serde::Serialize)]
struct CameraSecret {
  #[serde(rename = "v")]
  version: String,
  #[serde(rename = "cs")]
  secret: String,
}

pub fn generate_secluso_credentials(
  _app: &AppHandle,
  _run_id: Uuid,
  work_path: &Path,
  _repo: &str,
  _sig_keys: Option<&[SigKey]>,
  _github_token: Option<&str>,
) -> Result<()> {
  fs::create_dir_all(work_path).with_context(|| format!("creating work dir {}", work_path.display()))?;

  let crypto = OpenMlsRustCrypto::default();
  let secret = crypto
    .crypto()
    .random_vec(NUM_CAMERA_SECRET_BYTES)
    .context("Failed to generate camera secret bytes")?;

  let camera_secret = CameraSecret {
    version: CAMERA_SECRET_VERSION.to_string(),
    secret: base64_url::encode(&secret),
  };
  let qr_payload = serde_json::to_string(&camera_secret).context("Failed to serialize camera secret into JSON")?;

  fs::write(work_path.join("camera_secret"), &secret)
    .with_context(|| format!("writing {}", work_path.join("camera_secret").display()))?;

  let qr = QrCode::new(qr_payload).context("Failed to generate camera secret QR code")?;
  let qr_image = qr.render::<Luma<u8>>().build();
  qr_image
    .save(work_path.join("camera_secret_qrcode.png"))
    .with_context(|| format!("saving {}", work_path.join("camera_secret_qrcode.png").display()))?;

  Ok(())
}

pub fn generate_user_credentials_only(
  _app: &AppHandle,
  _run_id: Uuid,
  work_path: &Path,
  server_url: &str,
  _repo: &str,
  _sig_keys: Option<&[SigKey]>,
  _github_token: Option<&str>,
) -> Result<()> {
  fs::create_dir_all(work_path).with_context(|| format!("creating work dir {}", work_path.display()))?;

  let normalized_url = normalize_server_url(server_url)?;
  let (credentials, credentials_full) = create_user_credentials(normalized_url)?;

  fs::write(work_path.join("user_credentials"), credentials)
    .with_context(|| format!("writing {}", work_path.join("user_credentials").display()))?;
  fs::write(work_path.join("credentials_full"), &credentials_full)
    .with_context(|| format!("writing {}", work_path.join("credentials_full").display()))?;

  let qr = QrCode::new(credentials_full).context("Failed to generate user credentials QR code")?;
  let qr_image = qr.render::<Luma<u8>>().build();
  qr_image
    .save(work_path.join("user_credentials_qrcode.png"))
    .with_context(|| format!("saving {}", work_path.join("user_credentials_qrcode.png").display()))?;

  Ok(())
}

fn normalize_server_url(server_url: &str) -> Result<String> {
  let trimmed = server_url.trim();
  if trimmed.is_empty() {
    return Err(anyhow!("Server URL is empty."));
  }

  let parsed = Url::parse(trimmed).with_context(|| format!("Invalid server URL: {trimmed}"))?;
  match parsed.scheme() {
    "http" | "https" => {}
    other => return Err(anyhow!("Invalid server URL scheme: {other}")),
  }

  Ok(trimmed.trim_end_matches('/').to_string())
}
