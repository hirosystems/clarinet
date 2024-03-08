#[allow(unused_macros)]
#[macro_export]
macro_rules! impl_byte_array_newtype {
    ($thing:ident, $ty:ty, $len:expr) => {
        impl $thing {
            /// Instantiates from a hex string
            #[allow(dead_code)]
            pub fn from_hex(hex_str: &str) -> Result<$thing, clarity::util::HexError> {
                use clarity::util::hash::hex_bytes;
                let _hex_len = $len * 2;
                match (hex_str.len(), hex_bytes(hex_str)) {
                    (_hex_len, Ok(bytes)) => {
                        if bytes.len() != $len {
                            return Err(clarity::util::HexError::BadLength(hex_str.len()));
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
            pub fn from_vec(inp: &[u8]) -> Option<$thing> {
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
            pub fn from_vec_be(b: &[u8]) -> Option<$thing> {
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
                use clarity::util::hash::to_hex;
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
