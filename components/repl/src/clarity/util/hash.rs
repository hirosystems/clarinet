/*
 copyright: (c) 2013-2018 by Blockstack PBC, a public benefit corporation.

 This file is part of Blockstack.

 Blockstack is free software. You may redistribute or modify
 it under the terms of the GNU General Public License as published by
 the Free Software Foundation, either version 3 of the License or
 (at your option) any later version.

 Blockstack is distributed in the hope that it will be useful,
 but WITHOUT ANY WARRANTY, including without the implied warranty of
 MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 GNU General Public License for more details.

 You should have received a copy of the GNU General Public License
 along with Blockstack. If not, see <http://www.gnu.org/licenses/>.
*/

use super::pair::*;
use super::HexError;
use ripemd160::Ripemd160;
use sha2::{Digest, Sha256, Sha512, Sha512Trunc256};
use sha3::Keccak256;

use serde::de::Deserialize;
use serde::de::Error as de_Error;
use serde::ser::Error as ser_Error;
use serde::Serialize;

// borrowed from Andrew Poelstra's rust-bitcoin library
/// Convert a hexadecimal-encoded string to its corresponding bytes
pub fn hex_bytes(s: &str) -> Result<Vec<u8>, HexError> {
    let mut v = vec![];
    let mut iter = s.chars().pair();
    // Do the parsing
    iter.by_ref().fold(Ok(()), |e, (f, s)| {
        if e.is_err() {
            e
        } else {
            match (f.to_digit(16), s.to_digit(16)) {
                (None, _) => Err(HexError::BadCharacter(f)),
                (_, None) => Err(HexError::BadCharacter(s)),
                (Some(f), Some(s)) => {
                    v.push((f * 0x10 + s) as u8);
                    Ok(())
                }
            }
        }
    })?;
    // Check that there was no remainder
    match iter.remainder() {
        Some(_) => Err(HexError::BadLength(s.len())),
        None => Ok(v),
    }
}

/// Convert a slice of u8 to a hex string
pub fn to_hex(s: &[u8]) -> String {
    let r: Vec<String> = s.to_vec().iter().map(|b| format!("{:02x}", b)).collect();
    return r.join("");
}

/// Convert a vec of u8 to a hex string
pub fn bytes_to_hex(s: &Vec<u8>) -> String {
    to_hex(&s[..])
}

macro_rules! impl_serde_json_hex_string {
    ($name:ident, $len:expr) => {
        pub struct $name {}
        impl $name {
            pub fn json_serialize<S: serde::Serializer>(
                inst: &[u8; $len],
                s: S,
            ) -> Result<S::Ok, S::Error> {
                let hex_inst = to_hex(inst);
                s.serialize_str(&hex_inst.as_str())
            }

            pub fn json_deserialize<'de, D: serde::Deserializer<'de>>(
                d: D,
            ) -> Result<[u8; $len], D::Error> {
                let hex_inst = String::deserialize(d)?;
                let inst_bytes = hex_bytes(&hex_inst).map_err(de_Error::custom)?;

                match inst_bytes.len() {
                    $len => {
                        let mut byte_slice = [0u8; $len];
                        byte_slice.copy_from_slice(&inst_bytes);
                        Ok(byte_slice)
                    }
                    _ => Err(de_Error::custom(format!(
                        "Invalid hex string -- not {} bytes",
                        $len
                    ))),
                }
            }
        }
    };
}

impl_serde_json_hex_string!(Hash20, 20);

#[derive(Serialize, Deserialize)]
pub struct Hash160(
    #[serde(
        serialize_with = "Hash20::json_serialize",
        deserialize_with = "Hash20::json_deserialize"
    )]
    pub [u8; 20],
);
impl_array_newtype!(Hash160, u8, 20);
impl_array_hexstring_fmt!(Hash160);
impl_byte_array_newtype!(Hash160, u8, 20);

pub struct Keccak256Hash(pub [u8; 32]);
impl_array_newtype!(Keccak256Hash, u8, 32);
impl_array_hexstring_fmt!(Keccak256Hash);
impl_byte_array_newtype!(Keccak256Hash, u8, 32);

pub struct Sha256Sum(pub [u8; 32]);
impl_array_newtype!(Sha256Sum, u8, 32);
impl_array_hexstring_fmt!(Sha256Sum);
impl_byte_array_newtype!(Sha256Sum, u8, 32);

pub struct Sha512Sum(pub [u8; 64]);
impl_array_newtype!(Sha512Sum, u8, 64);
impl_array_hexstring_fmt!(Sha512Sum);
impl_byte_array_newtype!(Sha512Sum, u8, 64);

pub struct Sha512Trunc256Sum(pub [u8; 32]);
impl_array_newtype!(Sha512Trunc256Sum, u8, 32);
impl_array_hexstring_fmt!(Sha512Trunc256Sum);
impl_byte_array_newtype!(Sha512Trunc256Sum, u8, 32);

pub struct DoubleSha256(pub [u8; 32]);
impl_array_newtype!(DoubleSha256, u8, 32);
impl_array_hexstring_fmt!(DoubleSha256);
impl_byte_array_newtype!(DoubleSha256, u8, 32);

impl Hash160 {
    pub fn from_sha256(sha256_hash: &[u8; 32]) -> Hash160 {
        let mut rmd = Ripemd160::new();
        let mut ret = [0u8; 20];
        rmd.update(sha256_hash);
        ret.copy_from_slice(rmd.finalize().as_slice());
        Hash160(ret)
    }

    /// Create a hash by hashing some data
    /// (borrwed from Andrew Poelstra)
    pub fn from_data(data: &[u8]) -> Hash160 {
        let sha2_result = Sha256::digest(data);
        let ripe_160_result = Ripemd160::digest(sha2_result.as_slice());
        Hash160::from(ripe_160_result.as_slice())
    }
}

impl Sha512Sum {
    pub fn from_data(data: &[u8]) -> Sha512Sum {
        Sha512Sum::from(Sha512::digest(data).as_slice())
    }
}

impl Sha512Trunc256Sum {
    pub fn from_data(data: &[u8]) -> Sha512Trunc256Sum {
        Sha512Trunc256Sum::from(Sha512Trunc256::digest(data).as_slice())
    }
    pub fn from_hasher(hasher: Sha512Trunc256) -> Sha512Trunc256Sum {
        Sha512Trunc256Sum::from(hasher.finalize().as_slice())
    }
}

impl Keccak256Hash {
    pub fn from_data(data: &[u8]) -> Keccak256Hash {
        let mut tmp = [0u8; 32];
        let mut digest = Keccak256::new();
        digest.update(data);
        tmp.copy_from_slice(digest.finalize().as_slice());
        Keccak256Hash(tmp)
    }
}

impl Sha256Sum {
    pub fn from_data(data: &[u8]) -> Sha256Sum {
        let mut tmp = [0u8; 32];
        let mut sha2_1 = Sha256::new();
        sha2_1.update(data);
        tmp.copy_from_slice(sha2_1.finalize().as_slice());
        Sha256Sum(tmp)
    }
}

impl DoubleSha256 {
    pub fn from_data(data: &[u8]) -> DoubleSha256 {
        let mut tmp = [0u8; 32];

        let mut sha2 = Sha256::new();
        sha2.update(data);
        tmp.copy_from_slice(sha2.finalize().as_slice());

        let mut sha2_2 = Sha256::new();
        sha2_2.update(&tmp);
        tmp.copy_from_slice(sha2_2.finalize().as_slice());

        DoubleSha256(tmp)
    }
}
