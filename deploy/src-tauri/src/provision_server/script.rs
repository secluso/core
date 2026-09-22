//! SPDX-License-Identifier: GPL-3.0-or-later
const SERVER_SCRIPT: &str = include_str!("../../assets/server/provision_server.sh");

pub fn remote_provision_script() -> &'static str {
    SERVER_SCRIPT
}
