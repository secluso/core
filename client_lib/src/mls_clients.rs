//! Secluso list of MLS clients/users
//!
//! SPDX-License-Identifier: GPL-3.0-or-later

use crate::mls_client::MlsClient;

pub const NUM_MLS_CLIENTS: usize = 5;
pub static MLS_CLIENT_TAGS: [&str; NUM_MLS_CLIENTS] =
    ["motion", "thumbnail", "fcm", "livestream", "config"];

// indices for different clients
pub const MOTION: usize = 0;
pub const THUMBNAIL: usize = 1;
pub const FCM: usize = 2;
pub const LIVESTREAM: usize = 3;
pub const CONFIG: usize = 4;

pub type MlsClients = [MlsClient; NUM_MLS_CLIENTS];

// Used by the camera
// Motion, thumbnail, and FCM clients are shared between apps
// For livestream and config, there are dedicated clients per app
pub const NUM_COMMON_MLS_CLIENTS: usize = 3;
pub const NUM_DEDICATED_MLS_CLIENTS: usize = 2;

pub type MlsClientsCommon = [MlsClient; NUM_COMMON_MLS_CLIENTS];
pub type MlsClientsDedicated = [MlsClient; NUM_DEDICATED_MLS_CLIENTS];
pub const LIVESTREAM_DED: usize = 0;
pub const CONFIG_DED: usize = 1;

// Maximum time that we allow other group members to be offline (in seconds)
pub const MAX_OFFLINE_WINDOW: u64 = 24 * 60 * 60;
