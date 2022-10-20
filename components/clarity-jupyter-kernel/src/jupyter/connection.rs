use failure::Error;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use zmq;

pub type HmacSha256 = Hmac<Sha256>;

pub struct Connection {
    pub socket: zmq::Socket,
    pub mac: Option<HmacSha256>,
}

impl Connection {
    pub fn new(socket: zmq::Socket, key: &str) -> Result<Connection, Error> {
        let mac = if key.is_empty() {
            None
        } else {
            Some(HmacSha256::new_varkey(key.as_bytes()).expect("Shouldn't fail with HMAC"))
        };
        Ok(Connection { socket, mac })
    }
}
