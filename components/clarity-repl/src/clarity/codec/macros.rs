#[allow(unused_macros)]
macro_rules! impl_byte_array_message_codec {
    ($thing:ident, $len:expr) => {
        // use $crate::clarity::codec::{StacksMessageCodec, Error as codec_error};

        impl StacksMessageCodec for $thing {
            fn consensus_serialize<W: std::io::Write>(&self, fd: &mut W) -> Result<(), CodecError> {
                fd.write_all(self.as_bytes())
                    .map_err(CodecError::WriteError)
            }
            fn consensus_deserialize<R: std::io::Read>(fd: &mut R) -> Result<$thing, CodecError> {
                let mut buf = [0u8; ($len as usize)];
                fd.read_exact(&mut buf).map_err(CodecError::ReadError)?;
                let ret = $thing::from_bytes(&buf).expect("BUG: buffer is not the right size");
                Ok(ret)
            }
        }
    };
}
