//! SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Result;
use docopt::Docopt;
use semver::Version;
use serde::Deserialize;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

use secluso_update::{
    build_github_client, default_signers, download_and_verify_component, fetch_versioned_release,
    get_current_version, github_token_from_env, parse_sig_keys, server_version,
    write_current_version, Component, DEFAULT_OWNER_REPO,
};

const USAGE: &str = r#"
Secluso updater.

Usage:
  secluso-update --component COMPONENT [--once] [--bundle-path PATH] [--interval-secs N] [--github-timeout-secs N] [--restart-unit UNIT] [--github-repo <OWNER/REPO>] [--sig-key <NAME:GITHUB_USER>]...
  secluso-update (--help | -h)
  secluso-update (--version | -v)

Options:
  --component COMPONENT      Which single binary to update:
                             server | raspberry_camera_hub | config_tool
  --restart-unit UNIT        systemd unit to restart after install (optional).
                             If omitted, no service is restarted.
  --interval-secs N          Poll interval seconds [default: 60].
  --github-timeout-secs N    HTTP timeout seconds [default: 20].
  --github-repo <OWNER/REPO>  GitHub repo to poll for releases [default: secluso/secluso].
  --sig-key <NAME:GITHUB_USER>  Signature label + GitHub user (repeatable).
  --once                     Run a single update check then exit.
  --bundle-path PATH         Use a local bundle zip instead of downloading from GitHub.
  --version, -v              Show tool version.
  --help, -h                 Show this screen.
"#;

#[derive(Debug, Deserialize)]
struct Args {
    flag_component: String,
    flag_restart_unit: Option<String>,
    flag_interval_secs: u64,
    flag_github_timeout_secs: u64,
    flag_github_repo: String,
    flag_sig_key: Vec<String>,
    flag_once: bool,
    flag_bundle_path: Option<String>,
}

fn main() -> ! {
    let version = format!(
        "{}, version: {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    let args: Args = Docopt::new(USAGE)
        .map(|d| d.help(true))
        .map(|d| d.version(Some(version)))
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    if args.flag_once {
        println!("Going to check for updates.");
        if let Err(e) = check_update(&args) {
            eprintln!("Update check failed: {:#}", e);
            std::process::exit(1);
        }
        std::process::exit(0);
    }

    loop {
        println!("Going to check for updates.");
        if let Err(e) = check_update(&args) {
            eprintln!("Update check failed: {:#}", e);
        }
        sleep(Duration::from_secs(args.flag_interval_secs));
    }
}

fn check_update(args: &Args) -> Result<()> {
    // Parse component and signer policy first so we fail early on invalid operator input before
    // touching network or filesystem state.
    let component = Component::parse(&args.flag_component)?;
    let cli_signers = parse_sig_keys(&args.flag_sig_key)?;
    let signers = if cli_signers.is_empty() {
        default_signers()
    } else {
        cli_signers
    };

    // If no version marker exists yet, we use a default 0.0.0 that works as a placeholder
    let current_version = get_current_version(component).unwrap_or_else(|_| Version::new(0, 0, 0));
    println!("Current Version = {current_version}");

    let server_version = server_version().ok().flatten();
    let parsed_server_version = server_version
        .as_deref()
        .map(|s| s.trim().trim_start_matches('v'))
        .and_then(|s| Version::parse(s).ok());

    let proceed = parsed_server_version
        .is_some_and(|parsed_version| current_version < parsed_version);

    // We don't proceed unless the server version is greater than the component version
    if !proceed {
        let shown_version = server_version.as_deref().unwrap_or("UNKNOWN");
        println!("Server version is {}; update not needed yet", shown_version);
        return Ok(());
    }

    let github_token = github_token_from_env();
    let client = build_github_client(
        args.flag_github_timeout_secs,
        github_token.as_deref(),
        "secluso-updater",
    )?;

    // Allow repo override but keep a safe default
    let github_repo = if args.flag_github_repo.trim().is_empty() {
        DEFAULT_OWNER_REPO.to_string()
    } else {
        args.flag_github_repo.clone()
    };

    // Fetch the server's matching version release from github, so that we stay consistent.
    let release = fetch_versioned_release(&client, &github_repo, &server_version.unwrap())?;
    println!("Found Server Github Release Tag = {}", release.tag_name);
    if let Some(p) = &release.published_at {
        println!("Published At = {}", p);
    }

    // download_and_verify_component performs the full cryptographic verification pipeline and returns
    // only authenticated component bytes. allows focus below on atomic file placement.
    let bundle_path = args
        .flag_bundle_path
        .as_ref()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty());

    let verified = download_and_verify_component(
        &client,
        &release,
        component,
        std::env::consts::ARCH,
        bundle_path,
        &signers,
    )?;

    println!(
        "Verified component={} from {} ({} bytes)",
        args.flag_component,
        verified.component_path,
        verified.component_bytes.len()
    );

    let tmp_path = "/tmp/secluso-binary-tmp";
    let final_path = component.install_path();

    let final_dir = Path::new(&final_path)
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid install path: {}", final_path))?;

    fs::create_dir_all(final_dir)?;

    // Write to a temporary path first and move into place after optional stop/restart sequencing.
    // reduces partial-write risk on the final install path.
    fs::write(tmp_path, &verified.component_bytes)?;
    fs::set_permissions(tmp_path, fs::Permissions::from_mode(0o755))?;

    if let Some(unit) = args.flag_restart_unit.as_deref() {
        println!("Stopping unit: {}", unit);
        run(&format!("systemctl stop {}", shell_escape(unit)));
    }

    println!("Installing: {} -> {}", tmp_path, final_path);
    fs::rename(tmp_path, &final_path)?;

    if let Some(unit) = args.flag_restart_unit.as_deref() {
        println!("Starting unit: {}", unit);
        run(&format!("systemctl start {}", shell_escape(unit)));
    }

    // Persist version only after install has succeeded. Acts to gate future update checks (via the marker).
    write_current_version(component, verified.latest_version.clone())?;

    println!(
        "Updated to version {} (component={})",
        verified.latest_version, args.flag_component
    );
    Ok(())
}

// "best-effort" command runner used for service stop/start so updater can still finish installation
fn run(cmd: &str) {
    let _ = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

// Minimal shell escaping helper to safely embed unit names into sh -c commands.
fn shell_escape(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '@'))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', r#"'\''"#))
    }
}
