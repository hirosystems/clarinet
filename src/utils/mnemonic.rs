use bs58;
use hex;
use hmac::Hmac;
use pbkdf2::pbkdf2;
use ring::hmac::{Context, Key, HMAC_SHA512};
use ripemd160::Ripemd160;
use secp256k1::{PublicKey, SecretKey};
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

pub fn get_hardened_child_keypair(
    bip39_seed: &[u8],
    path: &[u32],
) -> Result<(Vec<u8>, String), String> {
    let (master_node_bytes, chain_code) = get_master_node_from_bip39_seed(&bip39_seed);
    let master_node = SecretKey::parse_slice(&master_node_bytes).unwrap();
    let (sk, _) = get_hardened_derivation(master_node, &chain_code, &path)?;
    let pk = PublicKey::from_secret_key(&sk);
    export_keypair(sk, pk)
}

// todo(ludo): Revisit this strategy. Should intensively use pk[..] instead.
pub fn export_keypair(
    secret_key: SecretKey,
    public_key: PublicKey,
) -> Result<(Vec<u8>, String), String> {
    let sk = secret_key.serialize();
    let pk = hex::encode(&public_key.serialize().to_vec());
    Ok((sk.to_vec(), pk))
}

pub fn get_master_node_from_bip39_seed(bip39_seed: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let key = Key::new(HMAC_SHA512, b"Bitcoin seed");
    let tag = ring::hmac::sign(&key, &bip39_seed);
    let mut master_node = tag.as_ref().to_vec();
    let chain_code = master_node.split_off(32);
    (master_node, chain_code)
}

pub fn get_hardened_derivation(
    root_key: SecretKey,
    root_code: &[u8],
    path: &[u32],
) -> Result<(SecretKey, Vec<u8>), String> {
    let mut parent_key = root_key;
    let mut parent_chain_code = root_code.to_vec();

    for index in path.iter() {
        // Hardened keys: [2^31: 2^32)
        let index = 2u32.pow(31) + index;
        // todo(ludo): check index in bound

        // Create signature
        let tag = {
            let key = Key::new(HMAC_SHA512, &parent_chain_code);
            let mut context = Context::with_key(&key);
            context.update(&[0x00]);
            context.update(&parent_key.serialize());
            context.update(&index.to_be_bytes());
            context.sign()
        };

        // Derive key
        let mut node_key = tag.as_ref().to_vec();
        let chain_code = node_key.split_off(32);
        let mut derived_key = SecretKey::parse_slice(&node_key).unwrap(); //.map_err(|_| { Error::KeyDerivationFailed });
        derived_key.tweak_add_assign(&parent_key).unwrap(); //.map_err(|_| { Error::KeyDerivationFailed })?;

        parent_key = derived_key;
        parent_chain_code = chain_code.to_vec();
    }
    Ok((parent_key, parent_chain_code))
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