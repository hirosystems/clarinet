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

//! Big unsigned integer types
//!
//! Implementation of a various large-but-fixed sized unsigned integer types.
//! The functions here are designed to be fast.
//!
/// Borrowed with gratitude from Andrew Poelstra's rust-bitcoin library
use std::fmt;

/// A trait which allows numbers to act as fixed-size bit arrays
pub trait BitArray {
    /// Is bit set?
    fn bit(&self, idx: usize) -> bool;

    /// Returns an array which is just the bits from start to end
    fn bit_slice(&self, start: usize, end: usize) -> Self;

    /// Bitwise and with `n` ones
    fn mask(&self, n: usize) -> Self;

    /// Trailing zeros
    fn trailing_zeros(&self) -> usize;

    /// Create all-zeros value
    fn zero() -> Self;

    /// Create value represeting one
    fn one() -> Self;

    /// Create value representing max
    fn max() -> Self;
}

macro_rules! construct_uint {
    ($name:ident, $n_words:expr) => {
        /// Little-endian large integer type
        #[repr(C)]
        pub struct $name(pub [u64; $n_words]);
        impl_array_newtype!($name, u64, $n_words);

        impl $name {
            /// Conversion to u32
            #[inline]
            pub fn low_u32(&self) -> u32 {
                let &$name(ref arr) = self;
                arr[0] as u32
            }

            /// Conversion to u64
            #[inline]
            pub fn low_u64(&self) -> u64 {
                let &$name(ref arr) = self;
                arr[0] as u64
            }

            /// Return the least number of bits needed to represent the number
            #[inline]
            pub fn bits(&self) -> usize {
                let &$name(ref arr) = self;
                for i in 1..$n_words {
                    if arr[$n_words - i] > 0 {
                        return (0x40 * ($n_words - i + 1))
                            - arr[$n_words - i].leading_zeros() as usize;
                    }
                }
                0x40 - arr[0].leading_zeros() as usize
            }

            /// Multiplication by u32
            pub fn mul_u32(self, other: u32) -> $name {
                let $name(ref arr) = self;
                let mut carry = [0u64; $n_words];
                let mut ret = [0u64; $n_words];
                for i in 0..$n_words {
                    let not_last_word = i < $n_words - 1;
                    let upper = other as u64 * (arr[i] >> 32);
                    let lower = other as u64 * (arr[i] & 0xFFFFFFFF);
                    if not_last_word {
                        carry[i + 1] += upper >> 32;
                    }
                    let (sum, overflow) = lower.overflowing_add(upper << 32);
                    ret[i] = sum;
                    if overflow && not_last_word {
                        carry[i + 1] += 1;
                    }
                }
                $name(ret) + $name(carry)
            }

            /// Create an object from a given unsigned 64-bit integer
            pub fn from_u64(init: u64) -> $name {
                let mut ret = [0; $n_words];
                ret[0] = init;
                $name(ret)
            }

            /// Create an object from a given signed 64-bit integer
            pub fn from_i64(init: i64) -> $name {
                assert!(init >= 0);
                $name::from_u64(init as u64)
            }

            /// Create an object from a given unsigned 128-bit integer
            pub fn from_u128(init: u128) -> $name {
                let mut ret = [0u64; $n_words];
                ret[0] = (init & 0xffffffffffffffffffffffffffffffff) as u64;
                ret[1] = (init >> 64) as u64;
                $name(ret)
            }

            /// max
            pub fn max() -> $name {
                let ret = [0xffffffffffffffff; $n_words];
                $name(ret)
            }

            /// as byte array
            pub fn to_u8_slice(&self) -> [u8; $n_words * 8] {
                let mut ret = [0u8; $n_words * 8];
                for i in 0..$n_words {
                    let bytes = self.0[i].to_le_bytes();
                    for j in 0..bytes.len() {
                        ret[i * 8 + j] = bytes[j];
                    }
                }
                ret
            }
        }

        impl ::std::ops::Add<$name> for $name {
            type Output = $name;

            fn add(self, other: $name) -> $name {
                let $name(ref me) = self;
                let $name(ref you) = other;
                let mut ret = [0u64; $n_words];
                let mut carry = [0u64; $n_words];
                let mut b_carry = false;
                for i in 0..$n_words {
                    ret[i] = me[i].wrapping_add(you[i]);
                    if i < $n_words - 1 && ret[i] < me[i] {
                        carry[i + 1] = 1;
                        b_carry = true;
                    }
                }
                if b_carry {
                    $name(ret) + $name(carry)
                } else {
                    $name(ret)
                }
            }
        }

        impl ::std::ops::Sub<$name> for $name {
            type Output = $name;

            #[inline]
            fn sub(self, other: $name) -> $name {
                self + !other + BitArray::one()
            }
        }

        impl ::std::ops::Mul<$name> for $name {
            type Output = $name;

            fn mul(self, other: $name) -> $name {
                let mut me = $name::zero();
                // TODO: be more efficient about this
                for i in 0..(2 * $n_words) {
                    let to_mul = (other >> (32 * i)).low_u32();
                    me = me + (self.mul_u32(to_mul) << (32 * i));
                }
                me
            }
        }

        impl ::std::ops::Div<$name> for $name {
            type Output = $name;

            fn div(self, other: $name) -> $name {
                let mut sub_copy = self;
                let mut shift_copy = other;
                let mut ret = [0u64; $n_words];

                let my_bits = self.bits();
                let your_bits = other.bits();

                // Check for division by 0
                assert!(your_bits != 0);

                // Early return in case we are dividing by a larger number than us
                if my_bits < your_bits {
                    return $name(ret);
                }

                // Bitwise long division
                let mut shift = my_bits - your_bits;
                shift_copy = shift_copy << shift;
                loop {
                    if sub_copy >= shift_copy {
                        ret[shift / 64] |= 1 << (shift % 64);
                        sub_copy = sub_copy - shift_copy;
                    }
                    shift_copy = shift_copy >> 1;
                    if shift == 0 {
                        break;
                    }
                    shift -= 1;
                }

                $name(ret)
            }
        }

        impl BitArray for $name {
            #[inline]
            fn bit(&self, index: usize) -> bool {
                let &$name(ref arr) = self;
                arr[index / 64] & (1 << (index % 64)) != 0
            }

            #[inline]
            fn bit_slice(&self, start: usize, end: usize) -> $name {
                (*self >> start).mask(end - start)
            }

            #[inline]
            fn mask(&self, n: usize) -> $name {
                let &$name(ref arr) = self;
                let mut ret = [0; $n_words];
                for i in 0..$n_words {
                    if n >= 0x40 * (i + 1) {
                        ret[i] = arr[i];
                    } else {
                        ret[i] = arr[i] & ((1 << (n - 0x40 * i)) - 1);
                        break;
                    }
                }
                $name(ret)
            }

            #[inline]
            fn trailing_zeros(&self) -> usize {
                let &$name(ref arr) = self;
                for i in 0..($n_words - 1) {
                    if arr[i] > 0 {
                        return (0x40 * i) + arr[i].trailing_zeros() as usize;
                    }
                }
                (0x40 * ($n_words - 1)) + arr[$n_words - 1].trailing_zeros() as usize
            }

            fn zero() -> $name {
                $name([0; $n_words])
            }
            fn one() -> $name {
                $name({
                    let mut ret = [0; $n_words];
                    ret[0] = 1;
                    ret
                })
            }
            fn max() -> $name {
                $name({
                    let ret = [0xffffffffffffffff; $n_words];
                    ret
                })
            }
        }

        impl ::std::default::Default for $name {
            fn default() -> $name {
                BitArray::zero()
            }
        }

        impl ::std::ops::BitAnd<$name> for $name {
            type Output = $name;

            #[inline]
            fn bitand(self, other: $name) -> $name {
                let $name(ref arr1) = self;
                let $name(ref arr2) = other;
                let mut ret = [0u64; $n_words];
                for i in 0..$n_words {
                    ret[i] = arr1[i] & arr2[i];
                }
                $name(ret)
            }
        }

        impl ::std::ops::BitXor<$name> for $name {
            type Output = $name;

            #[inline]
            fn bitxor(self, other: $name) -> $name {
                let $name(ref arr1) = self;
                let $name(ref arr2) = other;
                let mut ret = [0u64; $n_words];
                for i in 0..$n_words {
                    ret[i] = arr1[i] ^ arr2[i];
                }
                $name(ret)
            }
        }

        impl ::std::ops::BitOr<$name> for $name {
            type Output = $name;

            #[inline]
            fn bitor(self, other: $name) -> $name {
                let $name(ref arr1) = self;
                let $name(ref arr2) = other;
                let mut ret = [0u64; $n_words];
                for i in 0..$n_words {
                    ret[i] = arr1[i] | arr2[i];
                }
                $name(ret)
            }
        }

        impl ::std::ops::Not for $name {
            type Output = $name;

            #[inline]
            fn not(self) -> $name {
                let $name(ref arr) = self;
                let mut ret = [0u64; $n_words];
                for i in 0..$n_words {
                    ret[i] = !arr[i];
                }
                $name(ret)
            }
        }

        impl ::std::ops::Shl<usize> for $name {
            type Output = $name;

            fn shl(self, shift: usize) -> $name {
                let $name(ref original) = self;
                let mut ret = [0u64; $n_words];
                let word_shift = shift / 64;
                let bit_shift = shift % 64;
                for i in 0..$n_words {
                    // Shift
                    if bit_shift < 64 && i + word_shift < $n_words {
                        ret[i + word_shift] += original[i] << bit_shift;
                    }
                    // Carry
                    if bit_shift > 0 && i + word_shift + 1 < $n_words {
                        ret[i + word_shift + 1] += original[i] >> (64 - bit_shift);
                    }
                }
                $name(ret)
            }
        }

        impl ::std::ops::Shr<usize> for $name {
            type Output = $name;

            fn shr(self, shift: usize) -> $name {
                let $name(ref original) = self;
                let mut ret = [0u64; $n_words];
                let word_shift = shift / 64;
                let bit_shift = shift % 64;
                for i in word_shift..$n_words {
                    // Shift
                    ret[i - word_shift] += original[i] >> bit_shift;
                    // Carry
                    if bit_shift > 0 && i < $n_words - 1 {
                        ret[i - word_shift] += original[i + 1] << (64 - bit_shift);
                    }
                }
                $name(ret)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let &$name(ref data) = self;
                write!(f, "0x")?;
                for ch in data.iter().rev() {
                    write!(f, "{:016x}", ch)?;
                }
                Ok(())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                <dyn fmt::Debug>::fmt(self, f)
            }
        }
    };
}

construct_uint!(Uint256, 4);
construct_uint!(Uint512, 8);

impl Uint256 {
    /// Increment by 1
    #[inline]
    pub fn increment(&mut self) {
        let &mut Uint256(ref mut arr) = self;
        arr[0] += 1;
        if arr[0] == 0 {
            arr[1] += 1;
            if arr[1] == 0 {
                arr[2] += 1;
                if arr[2] == 0 {
                    arr[3] += 1;
                }
            }
        }
    }
}

impl Uint512 {
    /// from Uint256
    pub fn from_uint256(n: &Uint256) -> Uint512 {
        let mut tmp = [0u64; 8];
        for i in 0..4 {
            tmp[i] = n.0[i];
        }
        Uint512(tmp)
    }

    pub fn to_uint256(&self) -> Uint256 {
        let mut tmp = [0u64; 4];
        for i in 0..4 {
            tmp[i] = self.0[i];
        }
        Uint256(tmp)
    }
}
