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

use super::hash::hex_bytes;
use std::cell::RefCell;

// is this machine big-endian?
pub fn is_big_endian() -> bool {
    u32::from_be(0x1Au32) == 0x1Au32
}

/// Define a "named" enum, i.e., each variant corresponds
///  to a string literal, with a 1-1 mapping. You get EnumType::lookup_by_name
///  and EnumType.get_name() for free.
macro_rules! define_named_enum {
    ($Name:ident { $($Variant:ident($VarName:literal),)* }) =>
    {
        #[derive(Debug, Hash, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
        pub enum $Name {
            $($Variant),*,
        }
        impl $Name {
            pub const ALL: &'static [$Name] = &[$($Name::$Variant),*];
            pub const ALL_NAMES: &'static [&'static str] = &[$($VarName),*];

            pub fn lookup_by_name(name: &str) -> Option<Self> {
                match name {
                    $(
                        $VarName => Some($Name::$Variant),
                    )*
                    _ => None
                }
            }

            pub fn get_name(&self) -> String {
                match self {
                    $(
                        $Name::$Variant => $VarName.to_string(),
                    )*
                }
            }
        }
    }
}

/// Define a "u8" enum
///  gives you a try_from(u8) -> Option<Self> function
macro_rules! define_u8_enum {
    ($Name:ident { $($Variant:ident = $Val:literal),+ }) =>
    {
        #[derive(PartialEq)]
        #[repr(u8)]
        pub enum $Name {
            $($Variant = $Val),*,
        }
        impl $Name {
            pub const ALL: &'static [$Name] = &[$($Name::$Variant),*];

            pub fn to_u8(&self) -> u8 {
                match self {
                    $(
                        $Name::$Variant => $Val,
                    )*
                }
            }

            pub fn from_u8(v: u8) -> Option<Self> {
                match v {
                    $(
                        v if v == $Name::$Variant as u8 => Some($Name::$Variant),
                    )*
                    _ => None
                }
            }
        }
    }
}

/// Borrowed from Andrew Poelstra's rust-bitcoin
macro_rules! impl_array_newtype {
    ($thing:ident, $ty:ty, $len:expr) => {
        impl $thing {
            #[inline]
            #[allow(dead_code)]
            /// Converts the object to a raw pointer
            pub fn as_ptr(&self) -> *const $ty {
                let &$thing(ref dat) = self;
                dat.as_ptr()
            }

            #[inline]
            #[allow(dead_code)]
            /// Converts the object to a mutable raw pointer
            pub fn as_mut_ptr(&mut self) -> *mut $ty {
                let &mut $thing(ref mut dat) = self;
                dat.as_mut_ptr()
            }

            #[inline]
            #[allow(dead_code)]
            /// Returns the length of the object as an array
            pub fn len(&self) -> usize {
                $len
            }

            #[inline]
            #[allow(dead_code)]
            /// Returns whether the object, as an array, is empty. Always false.
            pub fn is_empty(&self) -> bool {
                false
            }

            #[inline]
            #[allow(dead_code)]
            /// Returns the underlying bytes.
            pub fn as_bytes(&self) -> &[$ty; $len] {
                &self.0
            }

            #[inline]
            #[allow(dead_code)]
            /// Returns the underlying bytes.
            pub fn to_bytes(&self) -> [$ty; $len] {
                self.0.clone()
            }

            #[inline]
            #[allow(dead_code)]
            /// Returns the underlying bytes.
            pub fn into_bytes(self) -> [$ty; $len] {
                self.0
            }
        }

        impl<'a> From<&'a [$ty]> for $thing {
            fn from(data: &'a [$ty]) -> $thing {
                assert_eq!(data.len(), $len);
                let mut ret = [0; $len];
                ret.copy_from_slice(&data[..]);
                $thing(ret)
            }
        }

        impl ::std::ops::Index<usize> for $thing {
            type Output = $ty;

            #[inline]
            fn index(&self, index: usize) -> &$ty {
                let &$thing(ref dat) = self;
                &dat[index]
            }
        }

        impl_index_newtype!($thing, $ty);

        impl PartialEq for $thing {
            #[inline]
            fn eq(&self, other: &$thing) -> bool {
                &self[..] == &other[..]
            }
        }

        impl Eq for $thing {}

        impl PartialOrd for $thing {
            #[inline]
            fn partial_cmp(&self, other: &$thing) -> Option<::std::cmp::Ordering> {
                Some(self.cmp(&other))
            }
        }

        impl Ord for $thing {
            #[inline]
            fn cmp(&self, other: &$thing) -> ::std::cmp::Ordering {
                // manually implement comparison to get little-endian ordering
                // (we need this for our numeric types; non-numeric ones shouldn't
                // be ordered anyway except to put them in BTrees or whatever, and
                // they don't care how we order as long as we're consisistent).
                for i in 0..$len {
                    if self[$len - 1 - i] < other[$len - 1 - i] {
                        return ::std::cmp::Ordering::Less;
                    }
                    if self[$len - 1 - i] > other[$len - 1 - i] {
                        return ::std::cmp::Ordering::Greater;
                    }
                }
                ::std::cmp::Ordering::Equal
            }
        }

        #[cfg_attr(feature = "clippy", allow(expl_impl_clone_on_copy))] // we don't define the `struct`, we have to explicitly impl
        impl Clone for $thing {
            #[inline]
            fn clone(&self) -> $thing {
                $thing::from(&self[..])
            }
        }

        impl Copy for $thing {}

        impl ::std::hash::Hash for $thing {
            #[inline]
            fn hash<H>(&self, state: &mut H)
            where
                H: ::std::hash::Hasher,
            {
                (&self[..]).hash(state);
            }

            fn hash_slice<H>(data: &[$thing], state: &mut H)
            where
                H: ::std::hash::Hasher,
            {
                for d in data.iter() {
                    (&d[..]).hash(state);
                }
            }
        }
    };
}

macro_rules! impl_index_newtype {
    ($thing:ident, $ty:ty) => {
        impl ::std::ops::Index<::std::ops::Range<usize>> for $thing {
            type Output = [$ty];

            #[inline]
            fn index(&self, index: ::std::ops::Range<usize>) -> &[$ty] {
                &self.0[index]
            }
        }

        impl ::std::ops::Index<::std::ops::RangeTo<usize>> for $thing {
            type Output = [$ty];

            #[inline]
            fn index(&self, index: ::std::ops::RangeTo<usize>) -> &[$ty] {
                &self.0[index]
            }
        }

        impl ::std::ops::Index<::std::ops::RangeFrom<usize>> for $thing {
            type Output = [$ty];

            #[inline]
            fn index(&self, index: ::std::ops::RangeFrom<usize>) -> &[$ty] {
                &self.0[index]
            }
        }

        impl ::std::ops::Index<::std::ops::RangeFull> for $thing {
            type Output = [$ty];

            #[inline]
            fn index(&self, _: ::std::ops::RangeFull) -> &[$ty] {
                &self.0[..]
            }
        }
    };
}

macro_rules! impl_array_hexstring_fmt {
    ($thing:ident) => {
        impl ::std::fmt::Debug for $thing {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                let &$thing(data) = self;
                for ch in data.iter() {
                    write!(f, "{:02x}", ch)?;
                }
                Ok(())
            }
        }
    };
}

#[allow(unused_macros)]
macro_rules! impl_byte_array_newtype {
    ($thing:ident, $ty:ty, $len:expr) => {
        impl $thing {
            /// Instantiates from a hex string
            #[allow(dead_code)]
            pub fn from_hex(hex_str: &str) -> Result<$thing, crate::clarity::util::HexError> {
                use crate::clarity::util::hash::hex_bytes;
                let _hex_len = $len * 2;
                match (hex_str.len(), hex_bytes(hex_str)) {
                    (_hex_len, Ok(bytes)) => {
                        if bytes.len() != $len {
                            return Err(crate::clarity::util::HexError::BadLength(hex_str.len()));
                        }
                        let mut ret = [0; $len];
                        ret.copy_from_slice(&bytes);
                        Ok($thing(ret))
                    }
                    (_, Err(e)) => Err(e),
                }
            }

            /// Instantiates from a slice of bytes
            #[allow(dead_code)]
            pub fn from_bytes(inp: &[u8]) -> Option<$thing> {
                match inp.len() {
                    $len => {
                        let mut ret = [0; $len];
                        ret.copy_from_slice(inp);
                        Some($thing(ret))
                    }
                    _ => None,
                }
            }

            /// Instantiates from a slice of bytes, converting to host byte order
            #[allow(dead_code)]
            pub fn from_bytes_be(inp: &[u8]) -> Option<$thing> {
                $thing::from_vec_be(&inp.to_vec())
            }

            /// Instantiates from a vector of bytes
            #[allow(dead_code)]
            pub fn from_vec(inp: &Vec<u8>) -> Option<$thing> {
                match inp.len() {
                    $len => {
                        let mut ret = [0; $len];
                        let bytes = &inp[..inp.len()];
                        ret.copy_from_slice(&bytes);
                        Some($thing(ret))
                    }
                    _ => None,
                }
            }

            /// Instantiates from a big-endian vector of bytes, converting to host byte order
            #[allow(dead_code)]
            pub fn from_vec_be(b: &Vec<u8>) -> Option<$thing> {
                match b.len() {
                    $len => {
                        let mut ret = [0; $len];
                        let bytes = &b[0..b.len()];
                        // flip endian to le if we are le
                        for i in 0..$len {
                            ret[$len - 1 - i] = bytes[i];
                        }
                        Some($thing(ret))
                    }
                    _ => None,
                }
            }

            /// Convert to a hex string
            #[allow(dead_code)]
            pub fn to_hex(&self) -> String {
                use crate::clarity::util::hash::to_hex;
                to_hex(&self.0)
            }
        }
        impl std::fmt::Display for $thing {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "{}", self.to_hex())
            }
        }
        impl std::convert::AsRef<[u8]> for $thing {
            fn as_ref(&self) -> &[u8] {
                &self.0
            }
        }
        impl std::convert::From<[u8; $len]> for $thing {
            fn from(o: [u8; $len]) -> Self {
                Self(o)
            }
        }
    };
}

#[allow(unused_macros)]
macro_rules! impl_byte_array_serde {
    ($thing:ident) => {
        impl serde::Serialize for $thing {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                let inst = self.to_hex();
                s.serialize_str(inst.as_str())
            }
        }

        impl<'de> serde::Deserialize<'de> for $thing {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<$thing, D::Error> {
                let inst_str = String::deserialize(d)?;
                $thing::from_hex(&inst_str).map_err(serde::de::Error::custom)
            }
        }
    };
}

// print debug statements while testing
#[allow(unused_macros)]
macro_rules! test_debug {
    ($($arg:tt)*) => (
        #[cfg(test)]
        {
            use std::env;
            if env::var("BLOCKSTACK_DEBUG") == Ok("1".to_string()) {
                println!($($arg)*);
            }
        }
    )
}

// enables/disables trace!() at compile-time
pub const TRACE_ENABLED: bool = true;

pub fn is_trace() -> bool {
    use std::env;
    TRACE_ENABLED && env::var("BLOCKSTACK_TRACE") == Ok("1".to_string())
}
