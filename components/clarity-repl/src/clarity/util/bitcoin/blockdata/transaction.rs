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

//! Bitcoin Transaction
//!
//! A transaction describes a transfer of money. It consumes previously-unspent
//! transaction outputs and produces new ones, satisfying the condition to spend
//! the old outputs (typically a digital signature with a specific key must be
//! provided) and defining the condition to spend the new ones. The use of digital
//! signatures ensures that coins cannot be spent by unauthorized parties.
//!
//! This module provides the structures and functions needed to support transactions.
//!

use std::default::Default;
use std::fmt;
use std::io::Write;

use crate::clarity::util::bitcoin::blockdata::script::Script;
use crate::clarity::util::bitcoin::network::encodable::{
    ConsensusDecodable, ConsensusEncodable, VarInt,
};
use crate::clarity::util::bitcoin::network::serialize::{
    self, serialize, BitcoinHash, SimpleDecoder, SimpleEncoder,
};
use crate::clarity::util::bitcoin::util::hash::Sha256dHash;

/// A reference to a transaction output
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct OutPoint {
    /// The referenced transaction's txid
    pub txid: Sha256dHash,
    /// The index of the referenced output in its transaction's vout
    pub vout: u32,
}
serde_struct_impl!(OutPoint, txid, vout);

impl OutPoint {
    /// Creates a "null" `OutPoint`.
    ///
    /// This value is used for coinbase transactions because they don't have
    /// any previous outputs.
    #[inline]
    pub fn null() -> OutPoint {
        OutPoint {
            txid: Default::default(),
            vout: u32::max_value(),
        }
    }

    /// Checks if an `OutPoint` is "null".
    ///
    /// # Examples
    ///
    /// ```rust
    /// use clarity_repl::clarity::util::bitcoin::blockdata::constants::genesis_block;
    /// use clarity_repl::clarity::util::bitcoin::network::constants::Network;
    ///
    /// let block = genesis_block(Network::Bitcoin);
    /// let tx = &block.txdata[0];
    ///
    /// // Coinbase transactions don't have any previous output.
    /// assert_eq!(tx.input[0].previous_output.is_null(), true);
    /// ```
    #[inline]
    pub fn is_null(&self) -> bool {
        *self == OutPoint::null()
    }
}

impl Default for OutPoint {
    fn default() -> Self {
        OutPoint::null()
    }
}

impl fmt::Display for OutPoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.txid, self.vout)
    }
}

/// A transaction input, which defines old coins to be consumed
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct TxIn {
    /// The reference to the previous output that is being used an an input
    pub previous_output: OutPoint,
    /// The script which pushes values on the stack which will cause
    /// the referenced output's script to accept
    pub script_sig: Script,
    /// The sequence number, which suggests to miners which of two
    /// conflicting transactions should be preferred, or 0xFFFFFFFF
    /// to ignore this feature. This is generally never used since
    /// the miner behaviour cannot be enforced.
    pub sequence: u32,
    /// Witness data: an array of byte-arrays.
    /// Note that this field is *not* (de)serialized with the rest of the TxIn in
    /// ConsensusEncodable/ConsennsusDecodable, as it is (de)serialized at the end of the full
    /// Transaction. It *is* (de)serialized with the rest of the TxIn in other (de)serializationn
    /// routines.
    pub witness: Vec<Vec<u8>>,
}
serde_struct_impl!(TxIn, previous_output, script_sig, sequence, witness);

/// A transaction output, which defines new coins to be created from old ones.
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct TxOut {
    /// The value of the output, in satoshis
    pub value: u64,
    /// The script which must satisfy for the output to be spent
    pub script_pubkey: Script,
}
serde_struct_impl!(TxOut, value, script_pubkey);

// This is used as a "null txout" in consensus signing code
impl Default for TxOut {
    fn default() -> TxOut {
        TxOut {
            value: 0xffffffffffffffff,
            script_pubkey: Script::new(),
        }
    }
}

/// A Bitcoin transaction, which describes an authenticated movement of coins
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct Transaction {
    /// The protocol version, should always be 1.
    pub version: u32,
    /// Block number before which this transaction is valid, or 0 for
    /// valid immediately.
    pub lock_time: u32,
    /// List of inputs
    pub input: Vec<TxIn>,
    /// List of outputs
    pub output: Vec<TxOut>,
}
serde_struct_impl!(Transaction, version, lock_time, input, output);

impl Transaction {
    /// Computes a "normalized TXID" which does not include any signatures.
    /// This gives a way to identify a transaction that is ``the same'' as
    /// another in the sense of having same inputs and outputs.
    pub fn ntxid(&self) -> Sha256dHash {
        let cloned_tx = Transaction {
            version: self.version,
            lock_time: self.lock_time,
            input: self
                .input
                .iter()
                .map(|txin| TxIn {
                    script_sig: Script::new(),
                    witness: vec![],
                    ..*txin
                })
                .collect(),
            output: self.output.clone(),
        };
        cloned_tx.bitcoin_hash()
    }

    /// Computes the txid. For non-segwit transactions this will be identical
    /// to the output of `BitcoinHash::bitcoin_hash()`, but for segwit transactions,
    /// this will give the correct txid (not including witnesses) while `bitcoin_hash`
    /// will also hash witnesses.
    pub fn txid(&self) -> Sha256dHash {
        use crate::clarity::util::bitcoin::util::hash::Sha256dEncoder;

        let mut enc = Sha256dEncoder::new();
        self.version.consensus_encode(&mut enc).unwrap();
        self.input.consensus_encode(&mut enc).unwrap();
        self.output.consensus_encode(&mut enc).unwrap();
        self.lock_time.consensus_encode(&mut enc).unwrap();
        enc.into_hash()
    }

    /// Computes a signature hash for a given input index with a given sighash flag.
    /// To actually produce a scriptSig, this hash needs to be run through an
    /// ECDSA signer, the SigHashType appended to the resulting sig, and a
    /// script written around this, but this is the general (and hard) part.
    ///
    /// *Warning* This does NOT attempt to support OP_CODESEPARATOR. In general
    /// this would require evaluating `script_pubkey` to determine which separators
    /// get evaluated and which don't, which we don't have the information to
    /// determine.
    ///
    /// # Panics
    /// Panics if `input_index` is greater than or equal to `self.input.len()`
    ///
    pub fn signature_hash(
        &self,
        input_index: usize,
        script_pubkey: &Script,
        sighash_u32: u32,
    ) -> Sha256dHash {
        assert!(input_index < self.input.len()); // Panic on OOB

        let (sighash, anyone_can_pay) =
            SigHashType::from_u32(sighash_u32).split_anyonecanpay_flag();

        // Special-case sighash_single bug because this is easy enough.
        if sighash == SigHashType::Single && input_index >= self.output.len() {
            return Sha256dHash::from(
                &[
                    1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0,
                ][..],
            );
        }

        // Build tx to sign
        let mut tx = Transaction {
            version: self.version,
            lock_time: self.lock_time,
            input: vec![],
            output: vec![],
        };
        // Add all inputs necessary..
        if anyone_can_pay {
            tx.input = vec![TxIn {
                previous_output: self.input[input_index].previous_output,
                script_sig: script_pubkey.clone(),
                sequence: self.input[input_index].sequence,
                witness: vec![],
            }];
        } else {
            tx.input = Vec::with_capacity(self.input.len());
            for (n, input) in self.input.iter().enumerate() {
                tx.input.push(TxIn {
                    previous_output: input.previous_output,
                    script_sig: if n == input_index {
                        script_pubkey.clone()
                    } else {
                        Script::new()
                    },
                    sequence: if n != input_index
                        && (sighash == SigHashType::Single || sighash == SigHashType::None)
                    {
                        0
                    } else {
                        input.sequence
                    },
                    witness: vec![],
                });
            }
        }
        // ..then all outputs
        tx.output = match sighash {
            SigHashType::All => self.output.clone(),
            SigHashType::Single => {
                let output_iter = self
                    .output
                    .iter()
                    .take(input_index + 1) // sign all outputs up to and including this one, but erase
                    .enumerate() // all of them except for this one
                    .map(|(n, out)| {
                        if n == input_index {
                            out.clone()
                        } else {
                            TxOut::default()
                        }
                    });
                output_iter.collect()
            }
            SigHashType::None => vec![],
            _ => unreachable!(),
        };
        // hash the result
        let mut raw_vec = serialize(&tx).unwrap();
        raw_vec.write_all(&sighash_u32.to_le_bytes()).unwrap();
        Sha256dHash::from_data(&raw_vec)
    }

    /// Gets the "weight" of this transaction, as defined by BIP141. For transactions with an empty
    /// witness, this is simply the consensus-serialized size times 4. For transactions with a
    /// witness, this is the non-witness consensus-serialized size multiplied by 3 plus the
    /// with-witness consensus-serialized size.
    #[inline]
    pub fn get_weight(&self) -> u64 {
        let mut input_weight = 0;
        let mut inputs_with_witnesses = 0;
        for input in &self.input {
            input_weight += 4
                * (32 + 4 + 4 + // outpoint (32+4) + nSequence
                VarInt(input.script_sig.len() as u64).encoded_length() +
                input.script_sig.len() as u64);
            if !input.witness.is_empty() {
                inputs_with_witnesses += 1;
                input_weight += VarInt(input.witness.len() as u64).encoded_length();
                for elem in &input.witness {
                    input_weight += VarInt(elem.len() as u64).encoded_length() + elem.len() as u64;
                }
            }
        }
        let mut output_size = 0;
        for output in &self.output {
            output_size += 8 + // value
                VarInt(output.script_pubkey.len() as u64).encoded_length() +
                output.script_pubkey.len() as u64;
        }
        let non_input_size =
        // version:
        4 +
        // count varints:
        VarInt(self.input.len() as u64).encoded_length() +
        VarInt(self.output.len() as u64).encoded_length() +
        output_size +
        // lock_time
        4;
        if inputs_with_witnesses == 0 {
            non_input_size * 4 + input_weight
        } else {
            non_input_size * 4 + input_weight + self.input.len() as u64 - inputs_with_witnesses + 2
        }
    }

    /// Is this a coin base transaction?
    pub fn is_coin_base(&self) -> bool {
        self.input.len() == 1 && self.input[0].previous_output.is_null()
    }
}

impl BitcoinHash for Transaction {
    fn bitcoin_hash(&self) -> Sha256dHash {
        use crate::clarity::util::bitcoin::util::hash::Sha256dEncoder;

        let mut enc = Sha256dEncoder::new();
        self.consensus_encode(&mut enc).unwrap();
        enc.into_hash()
    }
}

impl_consensus_encoding!(TxOut, value, script_pubkey);

impl<S: SimpleEncoder> ConsensusEncodable<S> for OutPoint {
    fn consensus_encode(&self, s: &mut S) -> Result<(), serialize::Error> {
        self.txid.consensus_encode(s)?;
        self.vout.consensus_encode(s)
    }
}
impl<D: SimpleDecoder> ConsensusDecodable<D> for OutPoint {
    fn consensus_decode(d: &mut D) -> Result<OutPoint, serialize::Error> {
        Ok(OutPoint {
            txid: ConsensusDecodable::consensus_decode(d)?,
            vout: ConsensusDecodable::consensus_decode(d)?,
        })
    }
}

impl<S: SimpleEncoder> ConsensusEncodable<S> for TxIn {
    fn consensus_encode(&self, s: &mut S) -> Result<(), serialize::Error> {
        self.previous_output.consensus_encode(s)?;
        self.script_sig.consensus_encode(s)?;
        self.sequence.consensus_encode(s)
    }
}
impl<D: SimpleDecoder> ConsensusDecodable<D> for TxIn {
    fn consensus_decode(d: &mut D) -> Result<TxIn, serialize::Error> {
        Ok(TxIn {
            previous_output: ConsensusDecodable::consensus_decode(d)?,
            script_sig: ConsensusDecodable::consensus_decode(d)?,
            sequence: ConsensusDecodable::consensus_decode(d)?,
            witness: vec![],
        })
    }
}

impl<S: SimpleEncoder> ConsensusEncodable<S> for Transaction {
    fn consensus_encode(&self, s: &mut S) -> Result<(), serialize::Error> {
        self.version.consensus_encode(s)?;
        let mut have_witness = false;
        for input in &self.input {
            if !input.witness.is_empty() {
                have_witness = true;
                break;
            }
        }
        if !have_witness {
            self.input.consensus_encode(s)?;
            self.output.consensus_encode(s)?;
        } else {
            0u8.consensus_encode(s)?;
            1u8.consensus_encode(s)?;
            self.input.consensus_encode(s)?;
            self.output.consensus_encode(s)?;
            for input in &self.input {
                input.witness.consensus_encode(s)?;
            }
        }
        self.lock_time.consensus_encode(s)
    }
}

impl<D: SimpleDecoder> ConsensusDecodable<D> for Transaction {
    fn consensus_decode(d: &mut D) -> Result<Transaction, serialize::Error> {
        let version: u32 = ConsensusDecodable::consensus_decode(d)?;
        let input: Vec<TxIn> = ConsensusDecodable::consensus_decode(d)?;
        // segwit
        if input.is_empty() {
            let segwit_flag: u8 = ConsensusDecodable::consensus_decode(d)?;
            match segwit_flag {
                // Empty tx
                0 => Ok(Transaction {
                    version: version,
                    input: input,
                    output: vec![],
                    lock_time: ConsensusDecodable::consensus_decode(d)?,
                }),
                // BIP144 input witnesses
                1 => {
                    let mut input: Vec<TxIn> = ConsensusDecodable::consensus_decode(d)?;
                    let output: Vec<TxOut> = ConsensusDecodable::consensus_decode(d)?;
                    for txin in input.iter_mut() {
                        txin.witness = ConsensusDecodable::consensus_decode(d)?;
                    }
                    if !input.is_empty() && input.iter().all(|input| input.witness.is_empty()) {
                        Err(serialize::Error::ParseFailed(
                            "witness flag set but no witnesses present",
                        ))
                    } else {
                        Ok(Transaction {
                            version: version,
                            input: input,
                            output: output,
                            lock_time: ConsensusDecodable::consensus_decode(d)?,
                        })
                    }
                }
                // We don't support anything else
                x => Err(serialize::Error::UnsupportedSegwitFlag(x)),
            }
        // non-segwit
        } else {
            Ok(Transaction {
                version: version,
                input: input,
                output: ConsensusDecodable::consensus_decode(d)?,
                lock_time: ConsensusDecodable::consensus_decode(d)?,
            })
        }
    }
}

/// Hashtype of a transaction, encoded in the last byte of a signature
/// Fixed values so they can be casted as integer types for encoding
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum SigHashType {
    /// 0x1: Sign all outputs
    All = 0x01,
    /// 0x2: Sign no outputs --- anyone can choose the destination
    None = 0x02,
    /// 0x3: Sign the output whose index matches this input's index. If none exists,
    /// sign the hash `0000000000000000000000000000000000000000000000000000000000000001`.
    /// (This rule is probably an unintentional C++ism, but it's consensus so we have
    /// to follow it.)
    Single = 0x03,
    /// 0x81: Sign all outputs but only this input
    AllPlusAnyoneCanPay = 0x81,
    /// 0x82: Sign no outputs and only this input
    NonePlusAnyoneCanPay = 0x82,
    /// 0x83: Sign one output and only this input (see `Single` for what "one output" means)
    SinglePlusAnyoneCanPay = 0x83,
}

impl SigHashType {
    /// Break the sighash flag into the "real" sighash flag and the ANYONECANPAY boolean
    fn split_anyonecanpay_flag(&self) -> (SigHashType, bool) {
        match *self {
            SigHashType::All => (SigHashType::All, false),
            SigHashType::None => (SigHashType::None, false),
            SigHashType::Single => (SigHashType::Single, false),
            SigHashType::AllPlusAnyoneCanPay => (SigHashType::All, true),
            SigHashType::NonePlusAnyoneCanPay => (SigHashType::None, true),
            SigHashType::SinglePlusAnyoneCanPay => (SigHashType::Single, true),
        }
    }

    /// Reads a 4-byte uint32 as a sighash type
    pub fn from_u32(n: u32) -> SigHashType {
        match n & 0x9f {
            // "real" sighashes
            0x01 => SigHashType::All,
            0x02 => SigHashType::None,
            0x03 => SigHashType::Single,
            0x81 => SigHashType::AllPlusAnyoneCanPay,
            0x82 => SigHashType::NonePlusAnyoneCanPay,
            0x83 => SigHashType::SinglePlusAnyoneCanPay,
            // catchalls
            x if x & 0x80 == 0x80 => SigHashType::AllPlusAnyoneCanPay,
            _ => SigHashType::All,
        }
    }

    /// Converts to a u32
    pub fn as_u32(&self) -> u32 {
        *self as u32
    }
}
