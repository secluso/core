//! SPDX-License-Identifier: GPL-3.0-or-later
use crate::provision_server::events::log_line;
use crate::provision_server::types::{ServerRuntimePlan, SshAuth, SshTarget};
use anyhow::{bail, Context, Result};
use ssh2::Session;
use std::io::{Read, Write};
use std::net::{IpAddr, ToSocketAddrs};
use tauri::AppHandle;
use uuid::Uuid;

pub const DEFAULT_SERVER_HTTP_PORT: u16 = 8000;
const MIN_DISK_KB: u64 = 3 * 1024 * 1024;
const WARN_MEM_KB: u64 = 768 * 1024;

pub struct PreflightReport {
  pub remote_has_bin: bool,
  pub remote_has_unit: bool,
  pub service_active: bool,
  pub installed_version: Option<String>,
  pub port_in_use: bool,
  pub remote_has_credentials_full: bool,
  pub remote_arch: String,
}

struct ExecResult {
  stdout: String,
  stderr: String,
  exit: i32,
}

pub fn run_preflight(
  app: &AppHandle,
  run_id: Uuid,
  step: &str,
  sess: &Session,
  target: &SshTarget,
  runtime: Option<&ServerRuntimePlan>,
  server_url: Option<&str>,
) -> Result<PreflightReport> {
  log_line(app, run_id, "info", Some(step), "Starting server preflight checks.");
  log_server_url_checks(app, run_id, step, target, runtime, server_url);

  let uname = remote_shell(sess, "uname -s", None)?;
  let kernel = uname.stdout.trim();
  if kernel != "Linux" {
    bail!("Unsupported remote OS: expected Linux, got {kernel}.");
  }
  log_line(app, run_id, "info", Some(step), format!("Remote OS kernel: {kernel}"));

  let os_release = remote_shell(sess, "if [ -f /etc/os-release ]; then cat /etc/os-release; fi", None)?;
  let pretty_name = parse_os_release_field(&os_release.stdout, "PRETTY_NAME");
  let distro_id = parse_os_release_field(&os_release.stdout, "ID");
  let distro_like = parse_os_release_field(&os_release.stdout, "ID_LIKE");
  if let Some(name) = pretty_name {
    log_line(app, run_id, "info", Some(step), format!("Remote distribution: {name}"));
  }
  if !remote_success(sess, "command -v apt-get >/dev/null 2>&1", None)? {
    bail!("This server does not provide apt-get. Automatic provisioning currently expects a Debian/Ubuntu-style Linux server.");
  }
  if distro_id.as_deref() != Some("debian")
    && distro_id.as_deref() != Some("ubuntu")
    && !distro_like
      .as_deref()
      .unwrap_or_default()
      .split_whitespace()
      .any(|v| v == "debian" || v == "ubuntu")
  {
    log_line(
      app,
      run_id,
      "warn",
      Some(step),
      "This distro is not clearly Debian/Ubuntu-like. Provisioning may still work because apt-get exists, but it is less certain.".to_string(),
    );
  }

  let pid1 = remote_shell(sess, "ps -p 1 -o comm= | tr -d '[:space:]'", None)?;
  if pid1.stdout.trim() != "systemd" {
    bail!("PID 1 is not systemd. Secluso provisioning expects a systemd-based Linux server.");
  }
  if !remote_success(sess, "command -v systemctl >/dev/null 2>&1", None)? {
    bail!("systemctl is missing. Secluso provisioning expects systemd utilities to be installed.");
  }

  verify_sudo_access(app, run_id, step, sess, target)?;

  let arch = remote_shell(sess, "uname -m", None)?;
  let arch = arch.stdout.trim();
  let remote_arch = match arch {
    "x86_64" => "x86_64".to_string(),
    "aarch64" | "arm64" => "aarch64".to_string(),
    other => bail!("Unsupported CPU architecture: {other}. Supported server architectures are x86_64 and aarch64."),
  };
  log_line(
    app,
    run_id,
    "info",
    Some(step),
    format!("Remote CPU architecture: {arch} (using bundle target {remote_arch})"),
  );

  let disk = remote_shell(sess, "df -Pk / | awk 'NR==2 {print $4}'", None)?;
  let avail_kb = parse_u64_field(disk.stdout.trim(), "available disk space")?;
  if avail_kb < MIN_DISK_KB {
    bail!(
      "Not enough free disk space on /. Need at least about 3 GiB available, found {:.1} MiB.",
      avail_kb as f64 / 1024.0
    );
  }
  log_line(
    app,
    run_id,
    "info",
    Some(step),
    format!("Free disk space on /: {:.1} GiB", avail_kb as f64 / 1024.0 / 1024.0),
  );

  let mem = remote_shell(sess, "awk '/MemAvailable:/ {print $2}' /proc/meminfo", None)?;
  let mem_kb = parse_u64_field(mem.stdout.trim(), "available memory")?;
  if mem_kb < WARN_MEM_KB {
    log_line(
      app,
      run_id,
      "warn",
      Some(step),
      format!(
        "Low available memory detected: {:.0} MiB. Secluso may still install, but downloads, updates, and first startup may be slower.",
        mem_kb as f64 / 1024.0
      ),
    );
  } else {
    log_line(
      app,
      run_id,
      "info",
      Some(step),
      format!("Available memory: {:.0} MiB", mem_kb as f64 / 1024.0),
    );
  }

  verify_outbound_network(app, run_id, step, sess)?;

  let remote_has_bin = remote_success(sess, "test -x /opt/secluso/bin/secluso-server", None)?;
  let remote_has_unit = remote_success(
    sess,
    "systemctl list-unit-files --type=service | awk '{print $1}' | grep -qx 'secluso-server.service'",
    None,
  )?;
  let service_active = remote_success(sess, "systemctl is-active --quiet secluso-server.service", None)?;
  let remote_has_credentials_full = remote_success(sess, "test -f /var/lib/secluso/credentials_full", None)?;
  let version = if remote_has_bin {
    let out = remote_shell(
      sess,
      "if [ -x /opt/secluso/bin/secluso-server ]; then /opt/secluso/bin/secluso-server --version 2>/dev/null | head -n1; fi",
      None,
    )?;
    let trimmed = out.stdout.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
  } else {
    None
  };

  if remote_has_bin || remote_has_unit {
    log_line(
      app,
      run_id,
      "info",
      Some(step),
      format!(
        "Existing install detected: binary={}, unit={}, active={}, version={}",
        remote_has_bin,
        remote_has_unit,
        service_active,
        version.clone().unwrap_or_else(|| "unknown".to_string())
      ),
    );
    if !remote_has_credentials_full {
      log_line(
        app,
        run_id,
        "warn",
        Some(step),
        "Existing install is missing /var/lib/secluso/credentials_full. This older layout is no longer upgraded in place; use Overwrite existing install for a clean reinstall.".to_string(),
      );
    }
  } else {
    log_line(app, run_id, "info", Some(step), "No existing Secluso server install detected.".to_string());
  }

  let listen_port = runtime.map(|value| value.listen_port).unwrap_or(DEFAULT_SERVER_HTTP_PORT);
  let port_probe = remote_with_optional_sudo(
    sess,
    target,
    &format!("ss -ltnpH | awk '$4 ~ /:{}$/ {{print}}'", listen_port),
    &format!("ss -ltnH | awk '$4 ~ /:{}$/ {{print}}'", listen_port),
  )?;
  let port_lines = port_probe
    .stdout
    .lines()
    .map(str::trim)
    .filter(|line| !line.is_empty())
    .collect::<Vec<_>>();
  let port_in_use = !port_lines.is_empty();
  if port_in_use {
    for line in &port_lines {
      log_line(app, run_id, "warn", Some(step), format!("Port {listen_port} listener: {line}"));
    }
    let occupied_by_secluso = port_probe.stdout.contains("secluso-server") || remote_has_bin || remote_has_unit;
    if !occupied_by_secluso {
      bail!("Port {listen_port} is already in use by another service. Automatic Secluso installs expect the selected listen port to be free.");
    }
    log_line(
      app,
      run_id,
      "warn",
      Some(step),
      format!("Port {listen_port} is already in use by an existing Secluso install or compatible listener."),
    );
  } else {
    log_line(app, run_id, "info", Some(step), format!("Port {listen_port} is free on the server."));
  }

  check_firewall(app, run_id, step, sess, runtime)?;

  Ok(PreflightReport {
    remote_has_bin,
    remote_has_unit,
    service_active,
    installed_version: version,
    port_in_use,
    remote_has_credentials_full,
    remote_arch,
  })
}

fn log_server_url_checks(
  app: &AppHandle,
  run_id: Uuid,
  step: &str,
  target: &SshTarget,
  runtime: Option<&ServerRuntimePlan>,
  server_url: Option<&str>,
) {
  let Some(server_url) = server_url.map(str::trim).filter(|v| !v.is_empty()) else {
    return;
  };

  match reqwest::Url::parse(server_url) {
    Ok(url) => {
      let exposure_mode = runtime.map(|value| value.exposure_mode.as_str()).unwrap_or("direct");
      let listen_port = runtime.map(|value| value.listen_port).unwrap_or(DEFAULT_SERVER_HTTP_PORT);
      if exposure_mode == "direct" && url.scheme() != "http" {
        log_line(
          app,
          run_id,
          "warn",
          Some(step),
          "Direct mode expects an http URL.".to_string(),
        );
      }
      let port = url.port_or_known_default().unwrap_or(listen_port);
      if exposure_mode == "direct" && port != listen_port {
        log_line(
          app,
          run_id,
          "warn",
          Some(step),
          "The configured public URL port does not match Secluso's direct listen port.".to_string(),
        );
      } else if exposure_mode == "proxy" {
        log_line(
          app,
          run_id,
          "info",
          Some(step),
          "Reverse proxy mode selected. The public URL port may differ from Secluso's local listen port.".to_string(),
        );
      }
      if let Some(host) = url.host_str() {
        if host.eq_ignore_ascii_case("localhost") {
          log_line(
            app,
            run_id,
            "warn",
            Some(step),
            "Credentials URL points at localhost. That only works on the same machine, not from the mobile app.".to_string(),
          );
        }
        if let Ok(ip) = host.parse::<IpAddr>() {
          if is_private_ip(ip) {
            log_line(
              app,
              run_id,
              "warn",
              Some(step),
              "Credentials URL points at a private/local address. Remote access will only work if the phone can reach that network or VPN."
                .to_string(),
            );
          }
        }
        if let Ok(target_ip) = target.host.parse::<IpAddr>() {
          let lookup_host = format!("{}:{}", host, port);
          match lookup_host.to_socket_addrs() {
            Ok(addrs) => {
              let resolved = addrs.map(|addr| addr.ip()).collect::<Vec<_>>();
              if !resolved.is_empty() && !resolved.contains(&target_ip) {
                log_line(
                  app,
                  run_id,
                  "warn",
                  Some(step),
                  "The configured credentials host does not resolve to the SSH target IP.".to_string(),
                );
              }
            }
            Err(err) => {
              let _ = err;
              log_line(
                app,
                run_id,
                "warn",
                Some(step),
                "Could not resolve the configured credentials host.".to_string(),
              );
            }
          }
        }
      }
    }
    Err(err) => {
      log_line(
        app,
        run_id,
        "warn",
        Some(step),
        format!("Could not parse the configured credentials URL: {err}"),
      );
    }
  }
}

fn verify_sudo_access(app: &AppHandle, run_id: Uuid, step: &str, sess: &Session, target: &SshTarget) -> Result<()> {
  if target.user == "root" {
    log_line(app, run_id, "info", Some(step), "SSH user is root; sudo check not needed.".to_string());
    return Ok(());
  }

  let (cmd, stdin, mode_label) = match target.sudo.mode.as_str() {
    "password" => {
      let pw = target.sudo.password.clone().unwrap_or_default();
      if pw.is_empty() {
        bail!("A sudo password is required for this login, but the sudo password field is empty.");
      }
      ("sudo -S -p '' true", Some(format!("{pw}\n")), "explicit sudo password")
    }
    "same" => match &target.auth {
      SshAuth::Password { password } if !password.is_empty() => {
        ("sudo -S -p '' true", Some(format!("{password}\n")), "same-as-login password")
      }
      _ => ("sudo -n true", None, "passwordless sudo"),
    },
    _ => ("sudo -n true", None, "passwordless sudo"),
  };

  let sudo = remote_shell(sess, cmd, stdin.as_deref())?;
  if sudo.exit != 0 {
    bail!(
      "sudo is not working with the current settings (mode: {mode_label}). {}",
      summarize_remote_failure(&sudo)
    );
  }
  log_line(app, run_id, "info", Some(step), format!("Verified sudo access using {mode_label}."));
  Ok(())
}

fn verify_outbound_network(app: &AppHandle, run_id: Uuid, step: &str, sess: &Session) -> Result<()> {
  let dns_checks = [
    (
      "api.github.com",
      "Future auto-updates may fail until the server can resolve api.github.com.",
    ),
    (
      "oauth2.googleapis.com",
      "Firebase push setup may fail until the server can resolve oauth2.googleapis.com.",
    ),
    (
      "fcm.googleapis.com",
      "Push notifications may fail until the server can resolve fcm.googleapis.com.",
    ),
  ];
  for (host, warning) in dns_checks {
    let probe = format!("getent ahostsv4 {host} >/dev/null 2>&1 || getent hosts {host} >/dev/null 2>&1");
    if remote_success(sess, &probe, None)? {
      log_line(app, run_id, "info", Some(step), format!("DNS lookup for {host} succeeded."));
    } else {
      log_line(app, run_id, "warn", Some(step), format!("DNS lookup for {host} failed. {warning}"));
    }
  }

  let https_checks = [
    (
      "https://api.github.com",
      "Future auto-updates may fail until the server can reach GitHub over HTTPS.",
    ),
    (
      "https://oauth2.googleapis.com",
      "Firebase push setup may fail until the server can reach Google OAuth over HTTPS.",
    ),
  ];
  for (url, warning) in https_checks {
    let probe = format!("if command -v curl >/dev/null 2>&1; then curl -fsSI --max-time 10 {url} >/dev/null; else exit 42; fi");
    let result = remote_shell(sess, &probe, None)?;
    match result.exit {
      0 => log_line(app, run_id, "info", Some(step), format!("Outbound HTTPS to {url} succeeded.")),
      42 => log_line(
        app,
        run_id,
        "warn",
        Some(step),
        format!("curl is not installed yet, so the remote HTTPS probe for {url} was skipped."),
      ),
      _ => log_line(app, run_id, "warn", Some(step), format!("Outbound HTTPS probe for {url} failed. {warning}")),
    }
  }

  Ok(())
}

fn check_firewall(app: &AppHandle, run_id: Uuid, step: &str, sess: &Session, runtime: Option<&ServerRuntimePlan>) -> Result<()> {
  let exposure_mode = runtime.map(|value| value.exposure_mode.as_str()).unwrap_or("direct");
  let listen_port = runtime.map(|value| value.listen_port).unwrap_or(DEFAULT_SERVER_HTTP_PORT);
  if exposure_mode == "proxy" {
    if remote_success(sess, "command -v nginx >/dev/null 2>&1 || command -v caddy >/dev/null 2>&1 || command -v apache2 >/dev/null 2>&1", None)? {
      log_line(
        app,
        run_id,
        "info",
        Some(step),
        format!("Reverse proxy mode selected. Make sure your existing proxy forwards to 127.0.0.1:{listen_port}."),
      );
    } else {
      log_line(
        app,
        run_id,
        "warn",
        Some(step),
        format!("Reverse proxy mode selected, but no common proxy binary was detected. Make sure something forwards traffic to 127.0.0.1:{listen_port}."),
      );
    }
    return Ok(());
  }

  if !remote_success(sess, "command -v ufw >/dev/null 2>&1", None)? {
    log_line(
      app,
      run_id,
      "warn",
      Some(step),
      format!("ufw is not installed. If your provider uses a cloud firewall or security group, make sure TCP port {listen_port} is allowed."),
    );
    return Ok(());
  }

  let ufw = remote_shell(sess, "ufw status", None)?;
  let status = ufw.stdout.to_lowercase();
  if status.contains("inactive") {
    log_line(
      app,
      run_id,
      "warn",
      Some(step),
      format!(
        "ufw is inactive. That is fine locally, but you may still need to open TCP port {listen_port} in your provider firewall or security group."
      ),
    );
  } else if status.contains(&listen_port.to_string()) && status.contains("allow") {
    log_line(app, run_id, "info", Some(step), format!("ufw appears to allow TCP port {listen_port}."));
  } else {
    log_line(
      app,
      run_id,
      "warn",
      Some(step),
      format!("ufw is active and does not obviously allow TCP port {listen_port}. Remote app access may fail until you open that port."),
    );
  }
  Ok(())
}

fn parse_os_release_field(contents: &str, key: &str) -> Option<String> {
  contents
    .lines()
    .find_map(|line| line.strip_prefix(&format!("{key}=")))
    .map(|value| value.trim_matches('"').to_string())
}

fn parse_u64_field(raw: &str, label: &str) -> Result<u64> {
  raw.parse::<u64>()
    .with_context(|| format!("Failed to parse {label} from '{raw}'"))
}

fn shell_escape(cmd: &str) -> String {
  cmd.replace('\'', r"'\''")
}

fn remote_success(sess: &Session, cmd: &str, stdin: Option<&str>) -> Result<bool> {
  Ok(remote_shell(sess, cmd, stdin)?.exit == 0)
}

fn remote_with_optional_sudo(sess: &Session, target: &SshTarget, sudo_cmd: &str, fallback_cmd: &str) -> Result<ExecResult> {
  if target.user == "root" {
    return remote_shell(sess, sudo_cmd, None);
  }

  match target.sudo.mode.as_str() {
    "password" => {
      let pw = target.sudo.password.clone().unwrap_or_default();
      if pw.is_empty() {
        return remote_shell(sess, fallback_cmd, None);
      }
      remote_shell(sess, sudo_cmd, Some(&format!("{pw}\n")))
    }
    "same" => match &target.auth {
      SshAuth::Password { password } if !password.is_empty() => {
        remote_shell(sess, sudo_cmd, Some(&format!("{password}\n")))
      }
      _ => {
        let res = remote_shell(sess, "sudo -n ss -ltnpH | awk '$4 ~ /:8000$/ {print}'", None)?;
        if res.exit == 0 { Ok(res) } else { remote_shell(sess, fallback_cmd, None) }
      }
    },
    _ => remote_shell(sess, fallback_cmd, None),
  }
}

fn remote_shell(sess: &Session, cmd: &str, stdin: Option<&str>) -> Result<ExecResult> {
  let full = format!("bash -lc '{}'", shell_escape(cmd));
  let mut channel = sess.channel_session().context("Failed to open SSH channel")?;
  channel.exec(&full).with_context(|| format!("Remote exec failed: {cmd}"))?;
  if let Some(stdin) = stdin {
    channel.write_all(stdin.as_bytes()).ok();
    channel.flush().ok();
  }
  channel.send_eof().ok();

  let mut stdout = String::new();
  let mut stderr = String::new();
  channel.read_to_string(&mut stdout).ok();
  channel.stderr().read_to_string(&mut stderr).ok();
  channel.wait_close().ok();
  let exit = channel.exit_status().unwrap_or(255);

  Ok(ExecResult { stdout, stderr, exit })
}

fn summarize_remote_failure(result: &ExecResult) -> String {
  let stderr = result.stderr.trim();
  if !stderr.is_empty() {
    return stderr.to_string();
  }
  let stdout = result.stdout.trim();
  if !stdout.is_empty() {
    return stdout.to_string();
  }
  format!("command exited with status {}", result.exit)
}

fn is_private_ip(ip: IpAddr) -> bool {
  match ip {
    IpAddr::V4(ip) => {
      ip.is_private() || ip.is_loopback() || ip.is_link_local() || ip.octets()[0] == 0
    }
    IpAddr::V6(ip) => ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local(),
  }
}
