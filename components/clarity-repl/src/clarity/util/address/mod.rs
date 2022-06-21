// Copyright (C) 2013-2020 Blockstack PBC, a public benefit corporation
// Copyright (C) 2020 Stacks Open Internet Foundation
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::error;
use std::fmt;

use super::secp256k1::Secp256k1PublicKey;

use super::bitcoin::blockdata::opcodes::All as btc_opcodes;
use super::bitcoin::blockdata::script::{Builder, Instruction, Script};

use crate::clarity::util::hash::Hash160;

use sha2::Digest;
use sha2::Sha256;

use std::convert::TryFrom;

pub mod b58;
pub mod c32;

#[derive(Debug)]
pub enum Error {
    InvalidCrockford32,
    InvalidVersion(u8),
    EmptyData,
    /// Invalid character encountered
    BadByte(u8),
    /// Checksum was not correct (expected, actual)
    BadChecksum(u32, u32),
    /// The length (in bytes) of the object was not correct
    /// Note that if the length is excessively long the provided length may be
    /// an estimate (and the checksum step may be skipped).
    InvalidLength(usize),
    /// Checked data was less than 4 bytes
    TooShort(usize),
    /// Any other error
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::InvalidCrockford32 => write!(f, "Invalid crockford 32 string"),
            Error::InvalidVersion(ref v) => write!(f, "Invalid version {}", v),
            Error::EmptyData => f.write_str("Empty data"),
            Error::BadByte(b) => write!(f, "invalid base58 character 0x{:x}", b),
            Error::BadChecksum(exp, actual) => write!(
                f,
                "base58ck checksum 0x{:x} does not match expected 0x{:x}",
                actual, exp
            ),
            Error::InvalidLength(ell) => write!(f, "length {} invalid for this base58 type", ell),
            Error::TooShort(_) => write!(f, "base58ck data not even long enough for a checksum"),
            Error::Other(ref s) => f.write_str(s),
        }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&dyn error::Error> {
        None
    }
    fn description(&self) -> &'static str {
        match *self {
            Error::InvalidCrockford32 => "Invalid crockford 32 string",
            Error::InvalidVersion(_) => "Invalid version",
            Error::EmptyData => "Empty data",
            Error::BadByte(_) => "invalid b58 character",
            Error::BadChecksum(_, _) => "invalid b58ck checksum",
            Error::InvalidLength(_) => "invalid length for b58 type",
            Error::TooShort(_) => "b58ck data less than 4 bytes",
            Error::Other(_) => "unknown b58 error",
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub enum AddressHashMode {
    // serialization modes for public keys to addresses.
    // We support four different modes due to legacy compatibility with Stacks v1 addresses:
    SerializeP2PKH = 0x00,  // hash160(public-key), same as bitcoin's p2pkh
    SerializeP2SH = 0x01,   // hash160(multisig-redeem-script), same as bitcoin's multisig p2sh
    SerializeP2WPKH = 0x02, // hash160(segwit-program-00(p2pkh)), same as bitcoin's p2sh-p2wpkh
    SerializeP2WSH = 0x03,  // hash160(segwit-program-00(public-keys)), same as bitcoin's p2sh-p2wsh
}

/// Given the u8 of an AddressHashMode, deduce the AddressHashNode
impl TryFrom<u8> for AddressHashMode {
    type Error = Error;

    fn try_from(value: u8) -> Result<AddressHashMode, Self::Error> {
        match value {
            x if x == AddressHashMode::SerializeP2PKH as u8 => Ok(AddressHashMode::SerializeP2PKH),
            x if x == AddressHashMode::SerializeP2SH as u8 => Ok(AddressHashMode::SerializeP2SH),
            x if x == AddressHashMode::SerializeP2WPKH as u8 => {
                Ok(AddressHashMode::SerializeP2WPKH)
            }
            x if x == AddressHashMode::SerializeP2WSH as u8 => Ok(AddressHashMode::SerializeP2WSH),
            _ => Err(Error::InvalidVersion(value)),
        }
    }
}

/// Internally, the Stacks blockchain encodes address the same as Bitcoin
/// single-sig address (p2pkh)
/// Get back the hash of the address
fn to_bits_p2pkh(pubk: &Secp256k1PublicKey) -> Hash160 {
    let key_hash = Hash160::from_data(&pubk.to_bytes());
    key_hash
}

/// Internally, the Stacks blockchain encodes address the same as Bitcoin
/// multi-sig address (p2sh)
fn to_bits_p2sh(num_sigs: usize, pubkeys: &Vec<Secp256k1PublicKey>) -> Hash160 {
    let mut bldr = Builder::new();
    bldr = bldr.push_int(num_sigs as i64);
    for pubk in pubkeys {
        bldr = bldr.push_slice(&pubk.to_bytes());
    }
    bldr = bldr.push_int(pubkeys.len() as i64);
    bldr = bldr.push_opcode(btc_opcodes::OP_CHECKMULTISIG);

    let script = bldr.into_script();
    let script_hash = Hash160::from_data(&script.as_bytes());
    script_hash
}

/// Internally, the Stacks blockchain encodes address the same as Bitcoin
/// single-sig address over p2sh (p2h-p2wpkh)
fn to_bits_p2sh_p2wpkh(pubk: &Secp256k1PublicKey) -> Hash160 {
    let key_hash = Hash160::from_data(&pubk.to_bytes());

    let bldr = Builder::new().push_int(0).push_slice(key_hash.as_bytes());

    let script = bldr.into_script();
    let script_hash = Hash160::from_data(&script.as_bytes());
    script_hash
}

/// Internally, the Stacks blockchain encodes address the same as Bitcoin
/// multisig address over p2sh (p2sh-p2wsh)
fn to_bits_p2sh_p2wsh(num_sigs: usize, pubkeys: &Vec<Secp256k1PublicKey>) -> Hash160 {
    let mut bldr = Builder::new();
    bldr = bldr.push_int(num_sigs as i64);
    for pubk in pubkeys {
        bldr = bldr.push_slice(&pubk.to_bytes());
    }
    bldr = bldr.push_int(pubkeys.len() as i64);
    bldr = bldr.push_opcode(btc_opcodes::OP_CHECKMULTISIG);

    let mut digest = Sha256::new();
    let mut d = [0u8; 32];

    digest.update(bldr.into_script().as_bytes());
    d.copy_from_slice(digest.finalize().as_slice());

    let ws = Builder::new().push_int(0).push_slice(&d).into_script();
    let ws_hash = Hash160::from_data(&ws.as_bytes());
    ws_hash
}

/// Convert a number of required signatures and a list of public keys into a byte-vec to hash to an
/// address.  Validity of the hash_flag vis a vis the num_sigs and pubkeys will _NOT_ be checked.
/// This is a low-level method.  Consider using StacksAdress::from_public_keys() if you can.
pub fn public_keys_to_address_hash(
    hash_flag: &AddressHashMode,
    num_sigs: usize,
    pubkeys: &Vec<Secp256k1PublicKey>,
) -> Hash160 {
    match *hash_flag {
        AddressHashMode::SerializeP2PKH => to_bits_p2pkh(&pubkeys[0]),
        AddressHashMode::SerializeP2SH => to_bits_p2sh(num_sigs, pubkeys),
        AddressHashMode::SerializeP2WPKH => to_bits_p2sh_p2wpkh(&pubkeys[0]),
        AddressHashMode::SerializeP2WSH => to_bits_p2sh_p2wsh(num_sigs, pubkeys),
    }
}
