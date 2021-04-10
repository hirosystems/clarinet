use bs58;
use hex;
use hmac::Hmac;
use pbkdf2::pbkdf2;
use ripemd160::Ripemd160;
use sha2::{Digest, Sha256};

pub fn get_bip39_seed_from_mnemonic(mnemonic: &str, password: &str) -> Result<Vec<u8>, String> {
    const PBKDF2_ROUNDS: u32 = 2048;
    const PBKDF2_BYTES: usize = 64;
    let salt = format!("mnemonic{}", password);
    let mut seed = vec![0u8; PBKDF2_BYTES];
    pbkdf2::<Hmac<sha2::Sha512>>(
        mnemonic.as_bytes(),
        salt.as_bytes(),
        PBKDF2_ROUNDS,
        &mut seed,
    );
    Ok(seed)
}

#[allow(dead_code)]
pub fn get_address_from_public_key(public_key: &str) -> Result<String, String> {
    let pub_key_hex = hex::decode(&public_key).unwrap();

    // SHA256
    let mut sha2 = Sha256::new();
    sha2.update(pub_key_hex);
    let pub_key_hashed = sha2.finalize();

    // RIPEMD160
    let mut rmd = Ripemd160::new();
    let mut pub_key_h160 = [0u8; 20];
    rmd.update(pub_key_hashed);
    pub_key_h160.copy_from_slice(rmd.finalize().as_slice());

    // Prepend version byte
    let version_byte = [0]; // MAINNET_SINGLESIG
    let v_pub_key_h160 = [&version_byte[..], &pub_key_h160[..]].concat();

    // Append checksum
    let mut sha2_1 = Sha256::new();
    sha2_1.update(v_pub_key_h160.clone());
    let mut sha2_2 = Sha256::new();
    sha2_2.update(sha2_1.finalize().as_slice());
    let checksum = sha2_2.finalize();
    let v_pub_key_h160_checksumed = [&v_pub_key_h160[..], &checksum[0..4]].concat();

    // Base58 encode
    Ok(bs58::encode(v_pub_key_h160_checksumed).into_string())
}