// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Networking protocols and utilities.

use anyhow::{anyhow, Result};
// use laminar::{Packet, Socket, SocketEvent};
use base64::{decode, encode};
use platform_dirs::AppDirs;
use rand::rngs::OsRng;
use rsa::{PublicKeyParts, RSAPrivateKey, RSAPublicKey};
use std::fs;
use std::io::prelude::*;

/// Loads the user's private key and generates a public key to go with it.
/// If there is no private key to be loaded, it will generate a new private key and then save it before handing it off to us.
pub fn load_keys() -> Result<(RSAPrivateKey, RSAPublicKey)> {
    // TODO should this be on a per-user basis?
    let app_dirs = AppDirs::new(Some("gridlocked"), false).ok_or(anyhow!("Could not get user app directories."))?;
    let key_dir = app_dirs.config_dir.join("keys");
    fs::create_dir_all(&key_dir)?;

    const PRIVATE_KEY_TAG: &str = "PRIVATE KEY\n";

    let key_file = key_dir.join("default.txt");
    let private_key = if key_file.exists() {
        log::info!("A private key file is present. It will be loaded in a moment.");

        let mut file = fs::File::open(key_file)?;
        let mut content = String::default();
        file.read_to_string(&mut content)?;

        let encoded = &content
            [(content.find(PRIVATE_KEY_TAG).ok_or(anyhow!("Could not find start of private key."))?) + PRIVATE_KEY_TAG.len()..];

        let data = decode(encoded)?;
        let n = rsa::BigUint::from_bytes_le(&data[0..3]);
        let e = rsa::BigUint::from_bytes_le(&data[4..7]);
        let d = rsa::BigUint::from_bytes_le(&data[8..11]);
        let mut primes = Vec::new();

        let primes_data = &data[12..];

        // Figure out how many bytes we could have.
        let length = primes_data.len() / 4;

        for index in 0..length {
            let localized_index = index * 4;
            primes.push(rsa::BigUint::from_bytes_le(&primes_data[localized_index..localized_index + 3]));
        }

        RSAPrivateKey::from_components(n, e, d, primes)
    } else {
        log::info!("There is no private key on this computer. Generating a new one. This will take a moment.\n");
        let mut rng = OsRng;
        let bits = 2048;

        let key = RSAPrivateKey::new(&mut rng, bits)?;
        let mut file_data = Vec::new();

        file_data.append(&mut key.n().to_bytes_le());
        file_data.append(&mut key.e().to_bytes_le());
        file_data.append(&mut key.d().to_bytes_le());

        for prime in key.primes() {
            file_data.append(&mut prime.to_bytes_le());
        }

        let mut file = fs::File::create(key_file)?;
        file.write_all(b"DO NOT SHARE THIS!\n")?;
        file.write_all(b"Do not give the content of this file to anyone!\n")?;
        file.write_all(b"Sharing this is worse than sharing your password.\n")?;
        file.write_all(
            b"This data is used to verify that you are really you. It's the equivalent of a username and password combined.\n",
        )?;
        file.write_all(b"Anyone with this data can impersonate you. There is no recovering an account that's private key has been lost or stolen.\n")?;
        file.write_all(PRIVATE_KEY_TAG.as_bytes())?;
        file.write_all(&encode(&file_data).as_bytes())?;

        key
    };

    let public_key = RSAPublicKey::from(&private_key);

    Ok((private_key, public_key))
}
