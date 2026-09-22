//! SPDX-License-Identifier: GPL-3.0-or-later
use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

pub fn shared_temp_dir(prefix: &str) -> Result<TempDir> {
    // keep temp dirs under a shared root so docker desktop does not warn
    let base = shared_temp_root();
    fs::create_dir_all(&base)
        .with_context(|| format!("create shared temp root at {}", base.display()))?;
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(&base)
        .with_context(|| format!("create temp dir under {}", base.display()))
}

pub fn shared_temp_root() -> PathBuf {
    if cfg!(target_os = "macos") {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Caches")
                .join("secluso-deploy");
        }
    }

    if let Ok(xdg) = env::var("XDG_CACHE_HOME") {
        return PathBuf::from(xdg).join("secluso-deploy");
    }

    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home).join(".cache").join("secluso-deploy");
    }

    env::temp_dir().join("secluso-deploy")
}
