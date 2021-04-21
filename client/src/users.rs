// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Management of user accounts.
//!
//! Users are managed in a completely local, decentralized manner. A centralized sync service may be provided in the future.
//! User accounts are verified using an RSA private key. Any server they wish to connect to will use a public key to identify
//! the user.

use anyhow::{anyhow, Context, Result};
use base64::{decode, encode};
use platform_dirs::AppDirs;
use rand::rngs::OsRng;
use rsa::{PublicKeyParts, RSAPrivateKey, RSAPublicKey};
use std::fs;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

struct UserProfile {
    folder: PathBuf,
    private_key: RSAPrivateKey,
    public_key: RSAPublicKey,
}

const PRIVATE_KEY_TAG: &str = "PRIVATE KEY\n";

impl UserProfile {
    /// Returns a list of possible user profiles. Note that this doesn't check
    /// that each user profile is fully valid. It just lists each directory under
    /// the profile's folder. You can use the load function to really verify that
    /// a user directory is valid.
    pub fn list_profiles() -> Result<Vec<String>> {
        let users_dir = Self::get_users_dir()?;

        let mut users = Vec::new();
        for entry in fs::read_dir(&users_dir)? {
            let directory = entry?;
            let path = directory.path();
            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    users.push(name.to_string_lossy().into_owned());
                }
            }
        }

        Ok(users)
    }

    /// Will load an already existing user profile on the local system.
    pub fn load(name: &str) -> Result<UserProfile> {
        let app_dirs = AppDirs::new(Some("gridlocked"), false).ok_or(anyhow!("Could not get user app directories."))?;
        let user_dir = Self::get_users_dir()?.join(name);
        let (private_key, public_key) = Self::load_keys(&user_dir).context("Error loading RSA key.")?;
        Ok(UserProfile { folder: user_dir, private_key, public_key })
    }

    /// Will create a new user profile on the local system.
    pub fn new(name: &str) -> Result<UserProfile> {
        // TODO there should be a general config file.
        let user_dir = Self::get_users_dir()?.join(name);
        if !user_dir.exists() {
            fs::create_dir_all(&user_dir)?;
            let (private_key, public_key) = Self::create_keys(&user_dir).context("Failed to create user's RSA keys")?;
            Ok(UserProfile { folder: user_dir, private_key, public_key })
        } else {
            Err(anyhow!("User directory \"{}\" already exists. If you wish to re-create it, delete it first.", name))
        }
    }

    /// Returns the path to the folder that should contain all user profiles.
    /// Will create the directory if needed.
    fn get_users_dir() -> Result<PathBuf> {
        let app_dirs = AppDirs::new(Some("gridlocked"), false).ok_or(anyhow!("Could not get user app directories."))?;
        let path = app_dirs.config_dir.join("users");
        fs::create_dir_all(&path)?;

        Ok(path)
    }

    /// Creates a new private key for the user and generates a public key to go with it.
    /// Note taht this saves the private key to the user's folder while it's at it.
    fn create_keys(user_folder: &Path) -> Result<(RSAPrivateKey, RSAPublicKey)> {
        let key_file = user_folder.join("private_key.txt");

        log::info!("There is no private key on this computer. Generating a new one. This will take a moment.\n");
        let mut rng = OsRng;
        let bits = 2048;

        let private_key = RSAPrivateKey::new(&mut rng, bits)?;
        let mut file_data = Vec::new();

        file_data.append(&mut private_key.n().to_bytes_le());
        file_data.append(&mut private_key.e().to_bytes_le());
        file_data.append(&mut private_key.d().to_bytes_le());

        for prime in private_key.primes() {
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

        let public_key = RSAPublicKey::from(&private_key);

        Ok((private_key, public_key))
    }

    /// Loads the user's private key and generates a public key to go with it.
    fn load_keys(user_folder: &Path) -> Result<(RSAPrivateKey, RSAPublicKey)> {
        log::info!("Loading user's RSA keys.");

        let key_file = user_folder.join("private_key.txt");
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

        let private_key = RSAPrivateKey::from_components(n, e, d, primes);
        let public_key = RSAPublicKey::from(&private_key);

        Ok((private_key, public_key))
    }
}
