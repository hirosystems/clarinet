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

#[macro_use]
pub mod macros;

pub mod address;
pub mod bitcoin;
pub mod c32;
pub mod hash;
pub mod pair;
pub mod retry;
pub mod secp256k1;
pub mod uint;

use std::error;
use std::fmt;
use std::thread;
use std::time;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::clarity::types::StandardPrincipalData;
use hash::Hash160;

use crate::clarity::types::PrincipalData;
use crate::clarity::util::secp256k1::Secp256k1PublicKey;
use address::public_keys_to_address_hash;
use address::AddressHashMode;

pub const C32_ADDRESS_VERSION_MAINNET_SINGLESIG: u8 = 22; // P
pub const C32_ADDRESS_VERSION_TESTNET_SINGLESIG: u8 = 26; // T

pub fn get_epoch_time_secs() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    return since_the_epoch.as_secs();
}

pub fn get_epoch_time_ms() -> u128 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    return since_the_epoch.as_millis();
}

pub fn sleep_ms(millis: u64) -> () {
    let t = time::Duration::from_millis(millis);
    thread::sleep(t);
}

/// Hex deserialization error
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum HexError {
    /// Length was not 64 characters
    BadLength(usize),
    /// Non-hex character in string
    BadCharacter(char),
}

impl fmt::Display for HexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            HexError::BadLength(n) => write!(f, "bad length {} for sha256d hex string", n),
            HexError::BadCharacter(c) => write!(f, "bad character {} in sha256d hex string", c),
        }
    }
}

impl error::Error for HexError {
    fn cause(&self) -> Option<&dyn error::Error> {
        None
    }
    fn description(&self) -> &str {
        match *self {
            HexError::BadLength(_) => "sha256d hex string non-64 length",
            HexError::BadCharacter(_) => "sha256d bad hex character",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy, Serialize, Deserialize, Hash)]
pub struct StacksAddress {
    pub version: u8,
    pub bytes: hash::Hash160,
}

impl From<StandardPrincipalData> for StacksAddress {
    fn from(o: StandardPrincipalData) -> StacksAddress {
        StacksAddress {
            version: o.0,
            bytes: Hash160(o.1),
        }
    }
}

impl StacksAddress {
    pub fn new(version: u8, hash: Hash160) -> StacksAddress {
        StacksAddress {
            version,
            bytes: hash,
        }
    }

    /// Generate an address from a given address hash mode, signature threshold, and list of public
    /// keys.  Only return an address if the combination given is supported.
    /// The version is may be arbitrary.
    pub fn from_public_key(version: u8, pubkey: Secp256k1PublicKey) -> Option<StacksAddress> {
        let hash_bits =
            public_keys_to_address_hash(&AddressHashMode::SerializeP2PKH, 1, &vec![pubkey]);
        Some(StacksAddress::new(version, hash_bits))
    }

    /// Convert to PrincipalData::Standard(StandardPrincipalData)
    pub fn to_account_principal(&self) -> PrincipalData {
        PrincipalData::Standard(StandardPrincipalData(
            self.version,
            self.bytes.as_bytes().clone(),
        ))
    }

    pub fn burn_address(mainnet: bool) -> StacksAddress {
        StacksAddress {
            version: if mainnet {
                C32_ADDRESS_VERSION_MAINNET_SINGLESIG
            } else {
                C32_ADDRESS_VERSION_TESTNET_SINGLESIG
            },
            bytes: Hash160([0u8; 20]),
        }
    }

    /// Generate an address from a given address hash mode, signature threshold, and list of public
    /// keys.  Only return an address if the combination given is supported.
    /// The version is may be arbitrary.
    pub fn from_public_keys(
        version: u8,
        hash_mode: &AddressHashMode,
        num_sigs: usize,
        pubkeys: &Vec<Secp256k1PublicKey>,
    ) -> Option<StacksAddress> {
        // must be sufficient public keys
        if pubkeys.len() < num_sigs {
            return None;
        }

        // address hash mode must be consistent with the number of keys
        match *hash_mode {
            AddressHashMode::SerializeP2PKH | AddressHashMode::SerializeP2WPKH => {
                // must be a single public key, and must require one signature
                if num_sigs != 1 || pubkeys.len() != 1 {
                    return None;
                }
            }
            _ => {}
        }

        // if segwit, then keys must all be compressed
        match *hash_mode {
            AddressHashMode::SerializeP2WPKH | AddressHashMode::SerializeP2WSH => {
                for pubkey in pubkeys {
                    if !pubkey.compressed() {
                        return None;
                    }
                }
            }
            _ => {}
        }

        let hash_bits = public_keys_to_address_hash(hash_mode, num_sigs, pubkeys);
        Some(StacksAddress::new(version, hash_bits))
    }
}

impl std::fmt::Display for StacksAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        c32::c32_address(self.version, self.bytes.as_bytes())
            .expect("Stacks version is not C32-encodable")
            .fmt(f)
    }
}
