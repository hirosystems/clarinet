// Rust Bitcoin Library
// Written in 2014 by
//     Andrew Poelstra <apoelstra@wpsoftware.net>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the CC0 Public Domain Dedication
// along with this software.
// If not, see <http://creativecommons.org/publicdomain/zero/1.0/>.
//

//! Script
//!
//! Scripts define Bitcoin's digital signature scheme: a signature is formed
//! from a script (the second half of which is defined by a coin to be spent,
//! and the first half provided by the spending transaction), and is valid
//! iff the script leaves `TRUE` on the stack after being evaluated.
//! Bitcoin's script is a stack-based assembly language similar in spirit to
//! Forth.
//!
//! This module provides the structures and functions needed to support scripts.
//!

use std::default::Default;
use std::{error, fmt};

use serde;

use crate::clarity::util::bitcoin::blockdata::opcodes;
use crate::clarity::util::bitcoin::network::encodable::{ConsensusDecodable, ConsensusEncodable};
use crate::clarity::util::bitcoin::network::serialize::{self, SimpleDecoder, SimpleEncoder};

// careful...
use crate::clarity::util::bitcoin::util::hash::Hash160;

use sha2::Digest;
use sha2::Sha256;

#[derive(Clone, Default, PartialOrd, Ord, PartialEq, Eq, Hash)]
/// A Bitcoin script
pub struct Script(Box<[u8]>);

impl fmt::Debug for Script {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut index = 0;

        f.write_str("Script(")?;
        while index < self.0.len() {
            let opcode = opcodes::All::from(self.0[index]);
            index += 1;

            let data_len = if let opcodes::Class::PushBytes(n) = opcode.classify() {
                n as usize
            } else {
                match opcode {
                    opcodes::All::OP_PUSHDATA1 => {
                        if self.0.len() < index + 1 {
                            f.write_str("<unexpected end>")?;
                            break;
                        }
                        match read_uint(&self.0[index..], 1) {
                            Ok(n) => {
                                index += 1;
                                n as usize
                            }
                            Err(_) => {
                                f.write_str("<bad length>")?;
                                break;
                            }
                        }
                    }
                    opcodes::All::OP_PUSHDATA2 => {
                        if self.0.len() < index + 2 {
                            f.write_str("<unexpected end>")?;
                            break;
                        }
                        match read_uint(&self.0[index..], 2) {
                            Ok(n) => {
                                index += 2;
                                n as usize
                            }
                            Err(_) => {
                                f.write_str("<bad length>")?;
                                break;
                            }
                        }
                    }
                    opcodes::All::OP_PUSHDATA4 => {
                        if self.0.len() < index + 4 {
                            f.write_str("<unexpected end>")?;
                            break;
                        }
                        match read_uint(&self.0[index..], 4) {
                            Ok(n) => {
                                index += 4;
                                n as usize
                            }
                            Err(_) => {
                                f.write_str("<bad length>")?;
                                break;
                            }
                        }
                    }
                    _ => 0,
                }
            };

            if index > 1 {
                f.write_str(" ")?;
            }
            // Write the opcode
            if opcode == opcodes::All::OP_PUSHBYTES_0 {
                f.write_str("OP_0")?;
            } else {
                write!(f, "{:?}", opcode)?;
            }
            // Write any pushdata
            if data_len > 0 {
                f.write_str(" ")?;
                if index + data_len <= self.0.len() {
                    for ch in &self.0[index..index + data_len] {
                        write!(f, "{:02x}", ch)?;
                    }
                    index += data_len;
                } else {
                    f.write_str("<push past end>")?;
                    break;
                }
            }
        }
        f.write_str(")")
    }
}

impl fmt::Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl fmt::LowerHex for Script {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for &ch in self.0.iter() {
            write!(f, "{:02x}", ch)?;
        }
        Ok(())
    }
}

impl fmt::UpperHex for Script {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for &ch in self.0.iter() {
            write!(f, "{:02X}", ch)?;
        }
        Ok(())
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
/// An object which can be used to construct a script piece by piece
pub struct Builder(Vec<u8>);
display_from_debug!(Builder);

/// Ways that a script might fail. Not everything is split up as
/// much as it could be; patches welcome if more detailed errors
/// would help you.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Error {
    /// Something did a non-minimal push; for more information see
    /// `https://github.com/bitcoin/bips/blob/master/bip-0062.mediawiki#Push_operators`
    NonMinimalPush,
    /// Some opcode expected a parameter, but it was missing or truncated
    EarlyEndOfScript,
    /// Tried to read an array off the stack as a number when it was more than 4 bytes
    NumericOverflow,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::NonMinimalPush => write!(f, "non-minimal datapush"),
            Error::EarlyEndOfScript => write!(f, "unexpected end of script"),
            Error::NumericOverflow => {
                write!(f, "numeric overflow (number on stack larger than 4 bytes)")
            }
        }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&dyn error::Error> {
        None
    }
}

/// Helper to encode an integer in script format
fn build_scriptint(n: i64) -> Vec<u8> {
    if n == 0 {
        return vec![];
    }

    let neg = n < 0;

    let mut abs = if neg { -n } else { n } as usize;
    let mut v = vec![];
    while abs > 0xFF {
        v.push((abs & 0xFF) as u8);
        abs >>= 8;
    }
    // If the number's value causes the sign bit to be set, we need an extra
    // byte to get the correct value and correct sign bit
    if abs & 0x80 != 0 {
        v.push(abs as u8);
        v.push(if neg { 0x80u8 } else { 0u8 });
    }
    // Otherwise we just set the sign bit ourselves
    else {
        abs |= if neg { 0x80 } else { 0 };
        v.push(abs as u8);
    }
    v
}

/// Helper to decode an integer in script format
/// Notice that this fails on overflow: the result is the same as in
/// bitcoind, that only 4-byte signed-magnitude values may be read as
/// numbers. They can be added or subtracted (and a long time ago,
/// multiplied and divided), and this may result in numbers which
/// can't be written out in 4 bytes or less. This is ok! The number
/// just can't be read as a number again.
/// This is a bit crazy and subtle, but it makes sense: you can load
/// 32-bit numbers and do anything with them, which back when mult/div
/// was allowed, could result in up to a 64-bit number. We don't want
/// overflow since that's suprising --- and we don't want numbers that
/// don't fit in 64 bits (for efficiency on modern processors) so we
/// simply say, anything in excess of 32 bits is no longer a number.
/// This is basically a ranged type implementation.
pub fn read_scriptint(v: &[u8]) -> Result<i64, Error> {
    let len = v.len();
    if len == 0 {
        return Ok(0);
    }
    if len > 4 {
        return Err(Error::NumericOverflow);
    }

    let (mut ret, sh) = v
        .iter()
        .fold((0, 0), |(acc, sh), n| (acc + ((*n as i64) << sh), sh + 8));
    if v[len - 1] & 0x80 != 0 {
        ret &= (1 << (sh - 1)) - 1;
        ret = -ret;
    }
    Ok(ret)
}

/// This is like "`read_scriptint` then map 0 to false and everything
/// else as true", except that the overflow rules don't apply.
#[inline]
pub fn read_scriptbool(v: &[u8]) -> bool {
    !(v.is_empty()
        || ((v[v.len() - 1] == 0 || v[v.len() - 1] == 0x80)
            && v.iter().rev().skip(1).all(|&w| w == 0)))
}

/// Read a script-encoded unsigned integer
pub fn read_uint(data: &[u8], size: usize) -> Result<usize, Error> {
    if data.len() < size {
        Err(Error::EarlyEndOfScript)
    } else {
        let mut ret = 0;
        for (i, item) in data.iter().take(size).enumerate() {
            ret += (*item as usize) << (i * 8);
        }
        Ok(ret)
    }
}

impl Script {
    /// Creates a new empty script
    pub fn new() -> Script {
        Script(vec![].into_boxed_slice())
    }

    /// The length in bytes of the script
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the script is the empty script
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the script data
    pub fn as_bytes(&self) -> &[u8] {
        &*self.0
    }

    /// Returns a copy of the script data
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.clone().into_vec()
    }

    /// Convert the script into a byte vector
    pub fn into_bytes(self) -> Vec<u8> {
        self.0.into_vec()
    }

    /// Compute the P2SH output corresponding to this redeem script
    pub fn to_p2sh(&self) -> Script {
        Builder::new()
            .push_opcode(opcodes::All::OP_HASH160)
            .push_slice(&Hash160::from_data(&self.0)[..])
            .push_opcode(opcodes::All::OP_EQUAL)
            .into_script()
    }

    /// Compute the P2WSH output corresponding to this witnessScript (aka the "witness redeem
    /// script")
    pub fn to_v0_p2wsh(&self) -> Script {
        let mut tmp = [0; 32];
        let mut sha2 = Sha256::new();
        sha2.update(&self.0);
        tmp.copy_from_slice(&sha2.finalize().as_slice());
        Builder::new().push_int(0).push_slice(&tmp).into_script()
    }

    /// Checks whether a script pubkey is a p2sh output
    #[inline]
    pub fn is_p2sh(&self) -> bool {
        self.0.len() == 23
            && self.0[0] == opcodes::All::OP_HASH160 as u8
            && self.0[1] == opcodes::All::OP_PUSHBYTES_20 as u8
            && self.0[22] == opcodes::All::OP_EQUAL as u8
    }

    /// Checks whether a script pubkey is a p2pkh output
    #[inline]
    pub fn is_p2pkh(&self) -> bool {
        self.0.len() == 25
            && self.0[0] == opcodes::All::OP_DUP as u8
            && self.0[1] == opcodes::All::OP_HASH160 as u8
            && self.0[2] == opcodes::All::OP_PUSHBYTES_20 as u8
            && self.0[23] == opcodes::All::OP_EQUALVERIFY as u8
            && self.0[24] == opcodes::All::OP_CHECKSIG as u8
    }

    /// Checks whether a script pubkey is a p2pkh output
    #[inline]
    pub fn is_p2pk(&self) -> bool {
        self.0.len() == 67
            && self.0[0] == opcodes::All::OP_PUSHBYTES_65 as u8
            && self.0[66] == opcodes::All::OP_CHECKSIG as u8
    }

    /// Checks whether a script pubkey is a p2wsh output
    #[inline]
    pub fn is_v0_p2wsh(&self) -> bool {
        self.0.len() == 34
            && self.0[0] == opcodes::All::OP_PUSHBYTES_0 as u8
            && self.0[1] == opcodes::All::OP_PUSHBYTES_32 as u8
    }

    /// Checks whether a script pubkey is a p2wpkh output
    #[inline]
    pub fn is_v0_p2wpkh(&self) -> bool {
        self.0.len() == 22
            && self.0[0] == opcodes::All::OP_PUSHBYTES_0 as u8
            && self.0[1] == opcodes::All::OP_PUSHBYTES_20 as u8
    }

    /// Check if this is an OP_RETURN output
    pub fn is_op_return(&self) -> bool {
        !self.0.is_empty() && (opcodes::All::from(self.0[0]) == opcodes::All::OP_RETURN)
    }

    /// Whether a script can be proven to have no satisfying input
    pub fn is_provably_unspendable(&self) -> bool {
        !self.0.is_empty()
            && (opcodes::All::from(self.0[0]).classify() == opcodes::Class::ReturnOp
                || opcodes::All::from(self.0[0]).classify() == opcodes::Class::IllegalOp)
    }

    /// Iterate over the script in the form of `Instruction`s, which are an enum covering
    /// opcodes, datapushes and errors. At most one error will be returned and then the
    /// iterator will end. To instead iterate over the script as sequence of bytes, treat
    /// it as a slice using `script[..]` or convert it to a vector using `into_bytes()`.
    pub fn iter(&self, enforce_minimal: bool) -> Instructions {
        Instructions {
            data: &self.0[..],
            enforce_minimal: enforce_minimal,
        }
    }
}

/// Creates a new script from an existing vector
impl From<Vec<u8>> for Script {
    fn from(v: Vec<u8>) -> Script {
        Script(v.into_boxed_slice())
    }
}

impl_index_newtype!(Script, u8);

/// A "parsed opcode" which allows iterating over a Script in a more sensible way
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Instruction<'a> {
    /// Push a bunch of data
    PushBytes(&'a [u8]),
    /// Some non-push opcode
    Op(opcodes::All),
    /// An opcode we were unable to parse
    Error(Error),
}

/// Iterator over a script returning parsed opcodes
pub struct Instructions<'a> {
    data: &'a [u8],
    enforce_minimal: bool,
}

impl<'a> Iterator for Instructions<'a> {
    type Item = Instruction<'a>;

    fn next(&mut self) -> Option<Instruction<'a>> {
        if self.data.is_empty() {
            return None;
        }

        match opcodes::All::from(self.data[0]).classify() {
            opcodes::Class::PushBytes(n) => {
                let n = n as usize;
                if self.data.len() < n + 1 {
                    self.data = &[]; // Kill iterator so that it does not return an infinite stream of errors
                    return Some(Instruction::Error(Error::EarlyEndOfScript));
                }
                if self.enforce_minimal {
                    if n == 1 && (self.data[1] == 0x81 || (self.data[1] > 0 && self.data[1] <= 16))
                    {
                        self.data = &[];
                        return Some(Instruction::Error(Error::NonMinimalPush));
                    }
                }
                let ret = Some(Instruction::PushBytes(&self.data[1..n + 1]));
                self.data = &self.data[n + 1..];
                ret
            }
            opcodes::Class::Ordinary(opcodes::Ordinary::OP_PUSHDATA1) => {
                if self.data.len() < 2 {
                    self.data = &[];
                    return Some(Instruction::Error(Error::EarlyEndOfScript));
                }
                let n = match read_uint(&self.data[1..], 1) {
                    Ok(n) => n,
                    Err(e) => {
                        self.data = &[];
                        return Some(Instruction::Error(e));
                    }
                };
                if self.data.len() < n + 2 {
                    self.data = &[];
                    return Some(Instruction::Error(Error::EarlyEndOfScript));
                }
                if self.enforce_minimal && n < 76 {
                    self.data = &[];
                    return Some(Instruction::Error(Error::NonMinimalPush));
                }
                let ret = Some(Instruction::PushBytes(&self.data[2..n + 2]));
                self.data = &self.data[n + 2..];
                ret
            }
            opcodes::Class::Ordinary(opcodes::Ordinary::OP_PUSHDATA2) => {
                if self.data.len() < 3 {
                    self.data = &[];
                    return Some(Instruction::Error(Error::EarlyEndOfScript));
                }
                let n = match read_uint(&self.data[1..], 2) {
                    Ok(n) => n,
                    Err(e) => {
                        self.data = &[];
                        return Some(Instruction::Error(e));
                    }
                };
                if self.enforce_minimal && n < 0x100 {
                    self.data = &[];
                    return Some(Instruction::Error(Error::NonMinimalPush));
                }
                if self.data.len() < n + 3 {
                    self.data = &[];
                    return Some(Instruction::Error(Error::EarlyEndOfScript));
                }
                let ret = Some(Instruction::PushBytes(&self.data[3..n + 3]));
                self.data = &self.data[n + 3..];
                ret
            }
            opcodes::Class::Ordinary(opcodes::Ordinary::OP_PUSHDATA4) => {
                if self.data.len() < 5 {
                    self.data = &[];
                    return Some(Instruction::Error(Error::EarlyEndOfScript));
                }
                let n = match read_uint(&self.data[1..], 4) {
                    Ok(n) => n,
                    Err(e) => {
                        self.data = &[];
                        return Some(Instruction::Error(e));
                    }
                };
                if self.enforce_minimal && n < 0x10000 {
                    self.data = &[];
                    return Some(Instruction::Error(Error::NonMinimalPush));
                }
                if self.data.len() < n + 5 {
                    self.data = &[];
                    return Some(Instruction::Error(Error::EarlyEndOfScript));
                }
                let ret = Some(Instruction::PushBytes(&self.data[5..n + 5]));
                self.data = &self.data[n + 5..];
                ret
            }
            // Everything else we can push right through
            _ => {
                let ret = Some(Instruction::Op(opcodes::All::from(self.data[0])));
                self.data = &self.data[1..];
                ret
            }
        }
    }
}

impl Builder {
    /// Creates a new empty script
    pub fn new() -> Builder {
        Builder(vec![])
    }

    /// The length in bytes of the script
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the script is the empty script
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Adds instructions to push an integer onto the stack. Integers are
    /// encoded as little-endian signed-magnitude numbers, but there are
    /// dedicated opcodes to push some small integers.
    pub fn push_int(mut self, data: i64) -> Builder {
        // We can special-case -1, 1-16
        if data == -1 || (data >= 1 && data <= 16) {
            self.0.push((data - 1 + opcodes::OP_TRUE as i64) as u8);
            self
        }
        // We can also special-case zero
        else if data == 0 {
            self.0.push(opcodes::OP_FALSE as u8);
            self
        }
        // Otherwise encode it as data
        else {
            self.push_scriptint(data)
        }
    }

    /// Adds instructions to push an integer onto the stack, using the explicit
    /// encoding regardless of the availability of dedicated opcodes.
    pub fn push_scriptint(self, data: i64) -> Builder {
        self.push_slice(&build_scriptint(data))
    }

    /// Adds instructions to push some arbitrary data onto the stack
    pub fn push_slice(mut self, data: &[u8]) -> Builder {
        // Start with a PUSH opcode
        match data.len() as u64 {
            n if n < opcodes::Ordinary::OP_PUSHDATA1 as u64 => {
                self.0.push(n as u8);
            }
            n if n < 0x100 => {
                self.0.push(opcodes::Ordinary::OP_PUSHDATA1 as u8);
                self.0.push(n as u8);
            }
            n if n < 0x10000 => {
                self.0.push(opcodes::Ordinary::OP_PUSHDATA2 as u8);
                self.0.push((n % 0x100) as u8);
                self.0.push((n / 0x100) as u8);
            }
            n if n < 0x100000000 => {
                self.0.push(opcodes::Ordinary::OP_PUSHDATA4 as u8);
                self.0.push((n % 0x100) as u8);
                self.0.push(((n / 0x100) % 0x100) as u8);
                self.0.push(((n / 0x10000) % 0x100) as u8);
                self.0.push((n / 0x1000000) as u8);
            }
            _ => panic!("tried to put a 4bn+ sized object into a script!"),
        }
        // Then push the acraw
        self.0.extend(data.iter().cloned());
        self
    }

    /// Adds a single opcode to the script
    pub fn push_opcode(mut self, data: opcodes::All) -> Builder {
        self.0.push(data as u8);
        self
    }

    /// Converts the `Builder` into an unmodifiable `Script`
    pub fn into_script(self) -> Script {
        Script(self.0.into_boxed_slice())
    }
}

/// Adds an individual opcode to the script
impl Default for Builder {
    fn default() -> Builder {
        Builder(vec![])
    }
}

/// Creates a new script from an existing vector
impl From<Vec<u8>> for Builder {
    fn from(v: Vec<u8>) -> Builder {
        Builder(v)
    }
}

impl_index_newtype!(Builder, u8);

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Script {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::fmt::{self, Formatter};

        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Script;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("a script")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let v: Vec<u8> = ::hex::decode(v).map_err(E::custom)?;
                Ok(Script::from(v))
            }

            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(v)
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(&v)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Script {
    /// User-facing serialization for `Script`.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{:x}", self))
    }
}

// Network serialization
impl<S: SimpleEncoder> ConsensusEncodable<S> for Script {
    #[inline]
    fn consensus_encode(&self, s: &mut S) -> Result<(), serialize::Error> {
        self.0.consensus_encode(s)
    }
}

impl<D: SimpleDecoder> ConsensusDecodable<D> for Script {
    #[inline]
    fn consensus_decode(d: &mut D) -> Result<Script, serialize::Error> {
        Ok(Script(ConsensusDecodable::consensus_decode(d)?))
    }
}
