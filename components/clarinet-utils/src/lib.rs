use std::str::FromStr;

use bip32::{DerivationPath, XPrv};
use bip39::{Language, Mnemonic};
use libsecp256k1::{PublicKey, SecretKey};
use rand::RngCore;

pub fn mnemonic_from_phrase(phrase: &str) -> Result<Mnemonic, String> {
    Mnemonic::parse_in(Language::English, phrase).map_err(|e| e.to_string())
}

pub fn random_mnemonic() -> Mnemonic {
    let mut entropy = [0u8; 16]; // 16 bytes = 128 bits = 12 words
    rand::thread_rng().fill_bytes(&mut entropy);
    Mnemonic::from_entropy_in(Language::English, &entropy).unwrap()
}

pub fn get_bip32_keys_from_mnemonic(
    phrase: &str,
    password: &str,
    derivation: &str,
) -> Result<(Vec<u8>, PublicKey), String> {
    let mnemonic = Mnemonic::parse_in(Language::English, phrase).map_err(|e| e.to_string())?;
    let seed_vec = mnemonic.to_seed(password);
    if seed_vec.len() != 64 {
        return Err("Seed must be 64 bytes".to_string());
    }
    let mut seed = [0u8; 64];
    seed.copy_from_slice(&seed_vec);
    let derivation_path = DerivationPath::from_str(derivation).map_err(|e| e.to_string())?;
    let xprv = XPrv::derive_from_path(seed, &derivation_path).map_err(|e| e.to_string())?;
    let secret_bytes = xprv.private_key().to_bytes();
    let secret_key = SecretKey::parse_slice(&secret_bytes).unwrap();
    let public_key = PublicKey::from_secret_key(&secret_key);
    Ok((secret_bytes.to_vec(), public_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mnemonic_from_phrase_12() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let mnemonic = mnemonic_from_phrase(phrase);
        assert!(mnemonic.is_ok());
        assert_eq!(mnemonic.unwrap().to_string(), phrase);
    }

    #[test]
    fn test_mnemonic_from_phrase_24() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
        let mnemonic = mnemonic_from_phrase(phrase);
        assert!(mnemonic.is_ok());
        assert_eq!(mnemonic.unwrap().to_string(), phrase);
    }

    #[test]
    fn test_random_mnemonic_12() {
        let mnemonic = random_mnemonic();
        let phrase = mnemonic.to_string();
        let words: Vec<&str> = phrase.split_whitespace().collect();
        assert_eq!(words.len(), 12);
    }

    #[test]
    fn test_get_bip32_keys_from_mnemonic_12() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let derivation = "m/44'/0'/0'/0/0";
        let result = get_bip32_keys_from_mnemonic(phrase, "", derivation);
        assert!(result.is_ok());
        let (secret, pubkey) = result.unwrap();
        assert_eq!(secret.len(), 32);
        assert_eq!(pubkey.serialize_compressed().len(), 33);
    }

    #[test]
    fn test_get_bip32_keys_from_mnemonic_24() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
        let derivation = "m/44'/0'/0'/0/0";
        let result = get_bip32_keys_from_mnemonic(phrase, "", derivation);
        assert!(result.is_ok());
        let (secret, pubkey) = result.unwrap();
        assert_eq!(secret.len(), 32);
        assert_eq!(pubkey.serialize_compressed().len(), 33);
    }
}
