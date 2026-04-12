//! Notification target persistence and dispatch helpers.
//!
//! SPDX-License-Identifier: GPL-3.0-or-later

use secluso_client_lib::http_client::{HttpClient, IosRelayBinding, NotificationTarget};
use std::fs;
use std::io;
use std::path::Path;
use url::Url;

const TARGET_FILENAME: &str = "notification_target.json";
const TRUSTED_IOS_RELAY_HOSTS: &[&str] = &["relay.secluso.com", "testing-relay.secluso.com"];

// Build the placeholder target we keep for iOS while no relay binding is available / after the current binding has been rejected.
fn ios_placeholder_target(platform: &str) -> NotificationTarget {
    NotificationTarget {
        platform: platform.to_string(),
        ios_relay_binding: None,
        unifiedpush_endpoint_url: None,
        unifiedpush_pub_key: None,
        unifiedpush_auth: None,
    }
}

// Mirror the server-side relay checks before the hub sends any outbound iOS request.
// Ensures a malicious/stale notification target cannot turn the hub into a generic HTTPS client.
fn validate_ios_relay_base_url(raw_url: &str) -> io::Result<Url> {
    let parsed = Url::parse(raw_url)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
    if parsed.scheme() != "https" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "iOS relay base URL must use https",
        ));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "iOS relay base URL must not include credentials",
        ));
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "iOS relay base URL must not include a query or fragment",
        ));
    }
    if parsed.path() != "/" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "iOS relay base URL must not include a path prefix",
        ));
    }

    let host = parsed.host_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "iOS relay base URL is missing a host",
        )
    })?;
    let port = parsed.port_or_known_default().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "iOS relay base URL is missing an https port",
        )
    })?;
    if port != 443 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "iOS relay base URL must use the default https port",
        ));
    }
    if !TRUSTED_IOS_RELAY_HOSTS
        .iter()
        .any(|allowed| host.eq_ignore_ascii_case(allowed))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Refusing unexpected iOS relay host: {host}"),
        ));
    }

    Ok(parsed)
}

fn validate_ios_relay_binding(binding: &IosRelayBinding) -> io::Result<()> {
    let relay_base = binding.relay_base_url.trim();
    if relay_base.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "iOS relay base URL is required",
        ));
    }
    validate_ios_relay_base_url(relay_base)?;

    Ok(())
}

pub fn persist_notification_target(state_dir: &str, target: &NotificationTarget) -> io::Result<()> {
    fs::create_dir_all(state_dir)?;
    let path = Path::new(state_dir).join(TARGET_FILENAME);
    let payload = serde_json::to_vec(target)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    fs::write(path, payload)
}

pub fn load_notification_target(state_dir: &str) -> io::Result<Option<NotificationTarget>> {
    let path = Path::new(state_dir).join(TARGET_FILENAME);
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(path)?;
    let target = serde_json::from_str::<NotificationTarget>(&raw)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    Ok(Some(target))
}

fn clear_notification_target(state_dir: &str) -> io::Result<()> {
    let path = Path::new(state_dir).join(TARGET_FILENAME);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn refresh_notification_target(
    state_dir: &str,
    http_client: &HttpClient,
) -> Option<NotificationTarget> {
    match http_client.fetch_notification_target() {
        Ok(Some(target)) => {
            if let Err(e) = persist_notification_target(state_dir, &target) {
                error!("Failed to persist notification target: {e}");
            }
            Some(target)
        }
        Ok(None) => {
            let cached = load_notification_target(state_dir).unwrap_or_else(|e| {
                error!("Failed to load cached notification target: {e}");
                None
            });
            if let Some(target) = cached {
                if target.platform.eq_ignore_ascii_case("ios") {
                    let placeholder = ios_placeholder_target(&target.platform);
                    if let Err(e) = persist_notification_target(state_dir, &placeholder) {
                        error!("Failed to persist iOS notification placeholder: {e}");
                    }
                    return Some(placeholder);
                }
            }
            if let Err(e) = clear_notification_target(state_dir) {
                error!("Failed to clear cached notification target: {e}");
            }
            None
        }
        Err(e) => {
            error!("Failed to fetch notification target: {e}");
            load_notification_target(state_dir).unwrap_or_else(|load_err| {
                error!("Failed to load cached notification target: {load_err}");
                None
            })
        }
    }
}

pub fn send_notification(
    state_dir: &str,
    http_client: &HttpClient,
    notification_msg: Vec<u8>,
) -> io::Result<()> {
    let target = refresh_notification_target(state_dir, http_client);

    if let Some(target) = target {
        if target.platform.eq_ignore_ascii_case("ios") {
            if let Some(binding) = target.ios_relay_binding.as_ref() {
                if let Err(e) = validate_ios_relay_binding(binding) {
                    let placeholder = ios_placeholder_target(&target.platform);
                    if let Err(clear_err) = persist_notification_target(state_dir, &placeholder) {
                        error!(
                            "Failed to persist iOS notification placeholder after relay validation failure: {clear_err}"
                        );
                    }
                    return Err(e);
                }

                let result = http_client.send_ios_notification(notification_msg, binding);
                if let Err(e) = result.as_ref() {
                    if e.to_string().contains("Relay error: 403") {
                        let placeholder = ios_placeholder_target(&target.platform);
                        if let Err(clear_err) = persist_notification_target(state_dir, &placeholder)
                        {
                            error!(
                                "Failed to persist iOS notification placeholder after relay 403: {clear_err}"
                            );
                        }
                    }
                }
                return result;
            }

            info!("Skipping iOS notification; relay binding is not available yet");
            return Ok(());
        }
    }

    http_client.send_fcm_notification(notification_msg)
}

#[cfg(test)]
mod tests {
    use super::{validate_ios_relay_base_url, validate_ios_relay_binding};
    use secluso_client_lib::http_client::IosRelayBinding;

    // Build an otherwise-valid relay binding and let each test vary only the relay base URL it wants to validate.
    fn ios_binding(relay_base_url: &str) -> IosRelayBinding {
        IosRelayBinding {
            relay_base_url: relay_base_url.to_string(),
            hub_token: "hub-token".to_string(),
            app_install_id: "install-id".to_string(),
            hub_id: "hub-id".to_string(),
            device_token: "device-token".to_string(),
            expires_at_epoch_ms: 1,
        }
    }

    #[test]
    // Tests that the camera hub accepts the public production relay.
    fn accepts_trusted_ios_relay_host() {
        validate_ios_relay_base_url("https://relay.secluso.com")
            .expect("trusted relay host should be accepted");
    }

    #[test]
    // Tests that server-side iOS relay checks reject unexpected relay hosts before the target can be persisted to the hub.
    fn rejects_untrusted_ios_relay_host() {
        let err = validate_ios_relay_base_url("https://evil.example")
            .expect_err("unexpected relay host should be rejected");

        assert!(err
            .to_string()
            .contains("Refusing unexpected iOS relay host"));
    }

    #[test]
    // Tests that the binding-level check rejects incomplete relay bindings before send_notification hands them to the HTTP client.
    fn rejects_empty_ios_relay_base_url() {
        let err = validate_ios_relay_binding(&ios_binding("   "))
            .expect_err("empty relay base URL should be rejected");

        assert!(err.to_string().contains("iOS relay base URL is required"));
    }
}
