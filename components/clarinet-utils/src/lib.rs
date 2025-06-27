use bip32::secp256k1::elliptic_curve::rand_core::OsRng;
use bip32::{Language, Mnemonic, XPrv};
use libsecp256k1::{PublicKey, SecretKey};

pub fn mnemonic_from_phrase(phrase: &str) -> Result<Mnemonic, String> {
    Mnemonic::new(phrase, bip32::Language::English).map_err(|e| e.to_string())
}

pub fn random_mnemonic() -> Mnemonic {
    Mnemonic::random(OsRng, Language::English)
}

pub fn get_bip32_keys_from_mnemonic(
    phrase: &str,
    password: &str,
    derivation: &str,
) -> Result<(Vec<u8>, PublicKey), String> {
    let mnemonic = Mnemonic::new(phrase, bip32::Language::English).map_err(|e| e.to_string())?;
    let xprv: XPrv =
        XPrv::derive_from_path(mnemonic.to_seed(password), &derivation.parse().unwrap())
            .map_err(|e| e.to_string())?;
    let secret_bytes = xprv.private_key().to_bytes();

    let secret_key = SecretKey::parse_slice(&secret_bytes).unwrap();
    let public_key = PublicKey::from_secret_key(&secret_key);

    Ok((secret_bytes.to_vec(), public_key))
}
