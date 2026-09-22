//! Secluso crypto provider
//!
//! SPDX-License-Identifier: GPL-3.0-or-later

//! Based on the OpenMLS client (openmls/cli).
//! MIT License.
//!
//! # OpenMLS Default Crypto Provider
//!
//! This is an implementation of the [`OpenMlsCryptoProvider`] trait to use with
//! OpenMLS.

use openmls_libcrux_crypto::CryptoProvider;
use openmls_rust_crypto::{MemoryStorage, RustCrypto};
use openmls_traits::OpenMlsProvider;
use std::fs::File;

pub struct OpenMlsRustPersistentCrypto {
    crypto: CryptoProvider,
    rand: RustCrypto,
    storage: MemoryStorage,
}

impl Default for OpenMlsRustPersistentCrypto {
    fn default() -> Self {
        Self {
            crypto: CryptoProvider::new().unwrap(),
            rand: RustCrypto::default(),
            storage: MemoryStorage::default(),
        }
    }
}

impl OpenMlsProvider for OpenMlsRustPersistentCrypto {
    type CryptoProvider = CryptoProvider;
    type RandProvider = RustCrypto;
    type StorageProvider = MemoryStorage;

    fn crypto(&self) -> &Self::CryptoProvider {
        &self.crypto
    }

    fn rand(&self) -> &Self::RandProvider {
        &self.rand
    }

    fn storage(&self) -> &Self::StorageProvider {
        &self.storage
    }
}

impl OpenMlsRustPersistentCrypto {
    pub fn save_keystore(&self, file: &File) -> Result<(), String> {
        self.storage.save_to_file(file)
    }

    pub fn load_keystore(&mut self, file: &File) -> Result<(), String> {
        self.storage.load_from_file(file)
    }
}
