//! Secluso app-camera pairing protocol.
//!
//! SPDX-License-Identifier: GPL-3.0-or-later

use openmls::prelude::KeyPackage;
use serde::{Deserialize, Serialize};
use qrcode::QrCode;
use image::Luma;
use std::fs::create_dir;
use std::io::Write;
use anyhow::Context;

use openmls_rust_crypto::OpenMlsRustCrypto;
use openmls_traits::random::OpenMlsRand;
use openmls_traits::OpenMlsProvider;

pub const NUM_SECRET_BYTES: usize = 72;
pub const CAMERA_SECRET_VERSION: &str = "v1.1";

// We version the QR code, store secret bytes as well (base64-url-encoded) as the Wi-Fi passphrase for Raspberry Pi cameras.
// Versioned QR codes can be helpful to ensure compatibility.
// Allows us to create backwards compatibility for previous QR versions without needing to re-generate QR codes again for users.
#[derive(Serialize, Deserialize)]
pub struct CameraSecret {
    #[serde(rename = "v", alias = "version")]
    pub version: String,

    // "cameras secret" = "cs", we shorten the fields to reduce the amount of bytes represented in the QrCode.
    // But this shouldn't be "s" to maintain separation from the user credentials qr code
    #[serde(rename = "cs", alias = "secret")]
    pub secret: String,
}

#[derive(Serialize, Deserialize, PartialEq)]
enum PairingMsgType {
    AppToCameraMsg,
    CameraToAppMsg,
}

#[derive(Serialize, Deserialize)]
struct PairingMsgContent {
    msg_type: PairingMsgType,
    key_package: KeyPackage,
}

#[derive(Serialize, Deserialize)]
struct PairingMsg {
    content_vec: Vec<u8>,
}

pub struct App {
    key_package: KeyPackage,
}


pub fn generate_ip_camera_secret(camera_name: &str) -> anyhow::Result<Vec<u8>> {
    let crypto = OpenMlsRustCrypto::default();
    let secret = crypto
        .crypto()
        .random_vec(NUM_SECRET_BYTES)
        .context("Failed to generate camera secret bytes")?;

    let camera_secret = CameraSecret {
        version: CAMERA_SECRET_VERSION.to_string(),
        secret: base64_url::encode(&secret),
    };

    let writeable_secret = serde_json::to_string(&camera_secret).context("Failed to serialize camera secret into JSON")?;

    // Save as QR code to be shown to the app
    let code = QrCode::new(writeable_secret.as_bytes()).context("Failed to generate QR code from camera secret bytes")?;
    let image = code.render::<Luma<u8>>().build();
    image
        .save(format!(
            "camera_{}_secret_qrcode.png",
            camera_name.replace(" ", "_").to_lowercase()
        )).context("Failed to save QR code image")?;

    Ok(secret)
}

pub fn generate_raspberry_camera_secret(dir: String) -> anyhow::Result<()> {
    let crypto = OpenMlsRustCrypto::default();
    let secret = crypto
        .crypto()
        .random_vec(NUM_SECRET_BYTES).context("Failed to generate camera secret bytes")?;

    let camera_secret = CameraSecret {
        version: CAMERA_SECRET_VERSION.to_string(),
        secret: base64_url::encode(&secret),
    };

    let writeable_secret = serde_json::to_string(&camera_secret).context("Failed to serialize camera secret into JSON")?;

    // Create the directory if it doesn't exist
    create_dir(dir.clone()).context("Failed to create directory (it may already exist)")?;

    // Save in a file to be given to the camera
    // The camera secret does not need to be versioned. We're not worried about the formatting ever changing.
    let mut file =
        std::fs::File::create(dir.clone() + "/camera_secret").context("Could not create file")?;
    file.write_all(&secret).context("Failed to write camera secret data to file")?;

    // Save as QR code to be shown to the app
    let code = QrCode::new(writeable_secret.clone()).context("Failed to generate QR code from camera secret bytes")?;
    let image = code.render::<Luma<u8>>().build();
    image
        .save(dir.clone() + "/camera_secret_qrcode.png").context("Failed to save QR code image")?;

    Ok(())
}


impl App {
    pub fn new(key_package: KeyPackage) -> Self {
        Self { key_package }
    }

    pub fn generate_msg_to_camera(&self) -> Vec<u8> {
        let msg_content = PairingMsgContent {
            msg_type: PairingMsgType::AppToCameraMsg,
            key_package: self.key_package.clone(),
        };
        let msg_content_vec = bincode::serialize(&msg_content).unwrap();

        let msg = PairingMsg {
            content_vec: msg_content_vec,
        };

        bincode::serialize(&msg).unwrap()
    }

    pub fn process_camera_msg(&self, camera_msg_vec: Vec<u8>) -> anyhow::Result<KeyPackage> {
        let camera_msg: PairingMsg = bincode::deserialize(&camera_msg_vec)?;

        let camera_msg_content: PairingMsgContent = bincode::deserialize(&camera_msg.content_vec)?;
        // Check the message type
        if camera_msg_content.msg_type != PairingMsgType::CameraToAppMsg {
            panic!("Received invalid pairing message!");
        }

        Ok(camera_msg_content.key_package)
    }
}

pub struct Camera {
    key_package: KeyPackage,
}

impl Camera {
    // FIXME: identical to App::new()
    pub fn new(key_package: KeyPackage) -> Self {
        Self { key_package }
    }

    pub fn process_app_msg_and_generate_msg_to_app(
        &self,
        app_msg_vec: Vec<u8>,
    ) -> anyhow::Result<(KeyPackage, Vec<u8>)> {
        let app_msg: PairingMsg = bincode::deserialize(&app_msg_vec).unwrap();

        let app_msg_content: PairingMsgContent =
            bincode::deserialize(&app_msg.content_vec).unwrap();

        // Check the message type
        if app_msg_content.msg_type != PairingMsgType::AppToCameraMsg {
            panic!("Received invalid pairing message!");
        }

        // Generate response
        let msg_content = PairingMsgContent {
            msg_type: PairingMsgType::CameraToAppMsg,
            key_package: self.key_package.clone(),
        };
        let msg_content_vec = bincode::serialize(&msg_content).unwrap();

        let resp_msg = PairingMsg {
            content_vec: msg_content_vec,
        };

        let resp_msg_vec = bincode::serialize(&resp_msg).unwrap();

        Ok((app_msg_content.key_package, resp_msg_vec))
    }
}
