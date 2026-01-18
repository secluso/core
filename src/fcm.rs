//! Secluso FCM.
//!
//! SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use plist::Value;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error;
use std::{fs, thread, time};

// Fixed bundle id used to locate the Firebase app
const BUNDLE_ID: &str = "com.secluso.mobile";

// Platform specific helpers for Firebase apps
enum Platform {
    Ios,
    Android,
}

impl Platform {
    fn apps_path(&self) -> &'static str {
        match self {
            Platform::Ios => "iosApps",
            Platform::Android => "androidApps",
        }
    }

    fn id_key(&self) -> &'static str {
        match self {
            Platform::Ios => "bundleId",
            Platform::Android => "packageName",
        }
    }

    fn display(&self) -> &'static str {
        match self {
            Self::Ios => "Secluso iOS",
            Self::Android => "Secluso Android",
        }
    }
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct App {
    appId: String,
    #[serde(default)]
    bundleId: Option<String>,
    #[serde(default)]
    packageName: Option<String>,
}

#[derive(Deserialize)]
struct ListApps {
    #[serde(default)]
    apps: Vec<App>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Operation {
    name: String,
    done: Option<bool>,
    #[serde(default)]
    error: Option<serde_json::Value>,
    #[serde(default)]
    response: Option<App>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ServiceAccountKey {
    #[serde(rename = "type")]
    key_type: String,
    project_id: String,
    private_key_id: String,
    private_key: String,
    client_email: String,
    client_id: String,
    auth_uri: String,
    token_uri: String,
    auth_provider_x509_cert_url: String,
    client_x509_cert_url: String,
}

#[derive(Debug, Serialize)]
struct Claims {
    iss: String,
    scope: String,
    aud: String,
    exp: usize,
    iat: usize,
}

// Configuration values returned to the client
#[derive(Serialize)]
pub struct ConfigResponse {
    api_key_ios: String,
    api_key_android: String,
    app_id_ios: String,
    app_id_android: String,
    messaging_sender_id: String,
    project_id: String,
    storage_bucket: String,
    bundle_id: String,
}

fn fetch_token(
    service_account_key: &ServiceAccountKey,
    client: &Client,
    scope: String,
) -> Result<String, Box<dyn Error>> {
    // Build an access token for Firebase API requests
    // Create the JWT claims
    let iat = Utc::now();
    let exp = iat + Duration::minutes(60);
    let claims = Claims {
        iss: service_account_key.client_email.clone(),
        scope: scope,
        aud: service_account_key.token_uri.clone(),
        exp: exp.timestamp() as usize,
        iat: iat.timestamp() as usize,
    };

    // Encode the JWT
    let header = Header::new(Algorithm::RS256);
    let private_key = service_account_key.private_key.replace("\\n", "\n");
    let encoding_key = EncodingKey::from_rsa_pem(private_key.as_bytes())?;
    let jwt = encode(&header, &claims, &encoding_key)?;

    // Obtain the OAuth 2.0 token
    let token_response: serde_json::Value = client
        .post(&service_account_key.token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &jwt),
        ])
        .send()?
        .error_for_status()?
        .json()?;

    Ok(token_response["access_token"]
        .as_str()
        .ok_or("Failed to get access_token")?
        .to_string())
}

fn fetch_app_identifier(
    client: &Client,
    service_account_key: &ServiceAccountKey,
    access_token: &str,
    arch: Platform,
) -> Result<Option<String>, Box<dyn Error>> {
    // Request the list of apps associated with the project
    let request_android_app_list_url = format!(
        "https://firebase.googleapis.com/v1beta1/projects/{}/{}?showDeleted=false&pageSize=10",
        service_account_key.project_id,
        arch.apps_path()
    );

    // Send the request for the app list
    let apps: ListApps = client
        .get(request_android_app_list_url)
        .bearer_auth(access_token.to_string())
        .send()
        .context("Failed to send App Identifier request")?
        .error_for_status()
        .context("Bad request on App Identifier request")?
        .json()
        .context("Failed to convert ListApps to JSON")?;

    for app in apps.apps {
        let ident = match arch {
            Platform::Android => app.packageName.as_deref(),
            Platform::Ios => app.bundleId.as_deref(),
        };

        if let Some(pkg_or_bundle) = ident {
            if pkg_or_bundle == BUNDLE_ID {
                return Ok(Some(app.appId));
            }
        }
    }

    Ok(None)
}

fn fetch_operation_status(
    client: &Client,
    access_token: &str,
    name: String,
) -> Result<Option<String>> {
    let request_operation_url = format!("https://firebase.googleapis.com/v1beta1/{}", name);

    // Check long running operation status
    let op: Operation = client
        .get(request_operation_url)
        .bearer_auth(access_token.to_string())
        .send()?
        .error_for_status()
        .context("Bad request on operation status")?
        .json()
        .context("Failed to parse operation JSON")?;

    if let Some(err) = op.error {
        anyhow::bail!("Operation failed: {}", err);
    }

    if op.done != Some(true) {
        return Ok(None);
    }

    if let Some(app) = op.response {
        return Ok(Some(app.appId));
    }

    Ok(None)
}

fn create_app(
    client: &Client,
    service_account_key: &ServiceAccountKey,
    access_token: &str,
    arch: Platform,
) -> Result<Option<String>, Box<dyn Error>> {
    // Build the app creation request for the project
    let request_android_app_list_url = format!(
        "https://firebase.googleapis.com/v1beta1/projects/{}/{}",
        service_account_key.project_id,
        arch.apps_path()
    );

    // Build the app creation payload

    let message = json!({
        "displayName": arch.display(),
        (arch.id_key()): BUNDLE_ID,
    });

    // Send the app creation request
    let response_text = client
        .post(request_android_app_list_url)
        .bearer_auth(access_token.to_string())
        .header("Content-Type", "application/json")
        .json(&message)
        .send()?
        .error_for_status()?
        .text();

    let json_body: serde_json::Value = serde_json::from_str(
        response_text
            .context("Failed to get response body")?
            .as_str(),
    )
    .context("JSON was not well-formatted")?;

    if let Some(operation_name) = json_body.get("name").and_then(|n| n.as_str()) {
        // Poll the operation until the app id is available
        for _ in 0..15 {
            thread::sleep(time::Duration::from_millis(1000));

            if let Ok(Some(app_id)) =
                fetch_operation_status(&client, access_token, operation_name.to_string())
            {
                return Ok(Some(app_id));
            }
        }
    }

    Ok(None)
}

fn send_config_request(
    client: &Client,
    service_account_key: &ServiceAccountKey,
    access_token: &str,
    arch: Platform,
    app_id: String,
) -> Result<String, Box<dyn Error>> {
    let request_operation_url = format!(
        "https://firebase.googleapis.com/v1beta1/projects/{}/{}/{}/config",
        service_account_key.project_id,
        arch.apps_path(),
        app_id
    );

    // Send the request for the config file
    let response = client
        .get(request_operation_url)
        .bearer_auth(access_token.to_string())
        .send()?
        .error_for_status()?;

    let json_body: serde_json::Value = serde_json::from_str(
        response
            .text()
            .context("Failed to get response body")?
            .as_str(),
    )
    .context("JSON was not well-formatted")?;
    let config_file_contents = json_body
        .get("configFileContents")
        .and_then(|v| v.as_str())
        .ok_or("Was not able to fetch the config contents")?;

    Ok(config_file_contents.to_string())
}

pub fn fetch_config() -> Result<ConfigResponse, Box<dyn Error>> {
    // Orchestrate app discovery creation and config retrieval
    let client = Client::new();

    // Read the service account key file
    let service_account_key: ServiceAccountKey = serde_json::from_str(
        &fs::read_to_string("service_account_key.json")
            .context("Failed to read service_account_key.json")?,
    )
    .context("Failed to parse service_account_key.json")?;

    // Read the service account key file
    let access_token = fetch_token(
        &service_account_key,
        &client,
        "https://www.googleapis.com/auth/firebase".to_string(),
    )?;
    let access_token = access_token.as_str();

    let mut pre_app_id_ios =
        fetch_app_identifier(&client, &service_account_key, access_token, Platform::Ios)?;
    let mut pre_app_id_android = fetch_app_identifier(
        &client,
        &service_account_key,
        access_token,
        Platform::Android,
    )?;

    if pre_app_id_ios.is_none() {
        pre_app_id_ios = create_app(&client, &service_account_key, access_token, Platform::Ios)?;
    }

    if pre_app_id_android.is_none() {
        pre_app_id_android = create_app(
            &client,
            &service_account_key,
            access_token,
            Platform::Android,
        )?;
    }

    let app_id_ios =
        pre_app_id_ios.context("Failure either creating or retrieving iOS app ID for Firebase")?;
    let app_id_android = pre_app_id_android
        .context("Failure either creating or retrieving Android app ID for Firebase")?;

    let ios_contents = send_config_request(
        &client,
        &service_account_key,
        access_token,
        Platform::Ios,
        app_id_ios.clone(),
    )?;
    let android_contents = send_config_request(
        &client,
        &service_account_key,
        access_token,
        Platform::Android,
        app_id_android.clone(),
    )?;

    let ios_contents_decoded = String::from_utf8(
        general_purpose::STANDARD
            .decode(ios_contents)
            .context("Failed to decode base64 file contents")?,
    )
    .context("Failed to convert base64 bytes into a string")?;
    let android_contents_decoded = String::from_utf8(
        general_purpose::STANDARD
            .decode(android_contents)
            .context("Failed to decode base64 file contents")?,
    )
    .context("Failed to convert base64 bytes into a string")?;

    let json_body_android: serde_json::Value =
        serde_json::from_str(android_contents_decoded.as_str())
            .context("JSON was not well-formatted")?;
    let project_info = json_body_android
        .get("project_info")
        .context("Failed to get project info")?;
    let messaging_sender_id = project_info
        .get("project_number")
        .and_then(|v| v.as_str())
        .context("Failed to parse project Number from Android firebase response")?;
    let storage_bucket = project_info
        .get("storage_bucket")
        .and_then(|v| v.as_str())
        .context("Failed to parse storage bucket from android firebase response")?;
    let project_id = service_account_key.project_id;

    let clients = json_body_android
        .get("client")
        .context("Failed to get 'client' from Android JSON config")?;
    let clients_array = clients
        .as_array()
        .context("'client' key in Android JSON was not an array")?;
    let first_client = clients_array
        .get(0)
        .context("clients_array in Android JSON had 0 keys")?;

    let api_key_android = first_client
        .get("api_key")
        .context("Failed to find api key in Android JSON")?
        .as_array()
        .context("Failed to convert api key field to array in Android JSON")?
        .get(0)
        .context("Failed to find any keys in Android JSON")?
        .get("current_key")
        .context("Failed to get current key in Android JSON")?
        .as_str()
        .context("Failed to convert current key to String in Android JSON")?;

    let ios_contents_parsed: Value = plist::from_bytes(ios_contents_decoded.as_bytes())
        .context("Failed to parse iOS plist from Firebase")?;
    let ios_contents_dict = ios_contents_parsed
        .as_dictionary()
        .context("Failed to convert iOS plist to dictionary")?;
    let api_key_ios = ios_contents_dict
        .get("API_KEY")
        .context("Failed to fetch API key from iOS plist from Firebase")?
        .as_string()
        .context("Failed to convert iOS API Key to a string")?;

    let response = ConfigResponse {
        api_key_ios: api_key_ios.to_string(),
        api_key_android: api_key_android.to_string(),
        app_id_android: app_id_android.to_string(),
        app_id_ios: app_id_ios.to_string(),
        messaging_sender_id: messaging_sender_id.to_string(),
        project_id: project_id,
        storage_bucket: storage_bucket.to_string(),
        bundle_id: BUNDLE_ID.to_string(),
    };

    Ok(response)
}

pub fn send_notification(device_token: String, msg: Vec<u8>) -> Result<(), Box<dyn Error>> {
    let client = Client::new();

    // Read the service account key file
    let service_account_key: ServiceAccountKey =
        serde_json::from_str(&fs::read_to_string("service_account_key.json")?)?;

    let access_token = fetch_token(
        &service_account_key,
        &client,
        "https://www.googleapis.com/auth/firebase.messaging".to_string(),
    )?;

    // The FCM endpoint for sending messages
    let fcm_url = format!(
        "https://fcm.googleapis.com/v1/projects/{}/messages:send",
        service_account_key.project_id
    );

    // Create the FCM message payload
    let message = json!({
        "message": {
            "token": device_token,
            "data": {
                "title": "",
                "body": general_purpose::STANDARD.encode(msg),
            },
            "android": {
                "priority": "high"
            }
        }
    });

    // Send the POST request
    let response = client
        .post(fcm_url)
        .bearer_auth(access_token)
        .header("Content-Type", "application/json")
        .json(&message)
        .send()?;

    // Check the response status
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(
            anyhow::anyhow!("Error: Failed to send notification. ({status}). {body}").into(),
        );
    }

    Ok(())
}
