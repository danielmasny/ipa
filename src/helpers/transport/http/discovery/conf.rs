use crate::helpers::{
    transport::http::discovery::{peer, Error, PeerDiscovery},
    HelperIdentity,
};
use std::collections::HashMap;

/// All config value necessary to discover other peer helpers of the MPC ring
#[cfg_attr(
    feature = "enable-serde",
    derive(serde::Deserialize),
    serde(transparent)
)]
pub struct Conf {
    peers_map: HashMap<HelperIdentity, peer::Config>,
}

impl Conf {
    /// Reads config from string. Expects config to be toml format.
    /// To read file, use `fs::read_to_string`
    ///
    /// # Errors
    /// if `input` is in an invalid format
    pub fn from_toml_str(input: &str) -> Result<Self, Error> {
        use config::{Config, File, FileFormat};

        let conf: Self = Config::builder()
            .add_source(File::from_str(input, FileFormat::Toml))
            .build()?
            .try_deserialize()?;

        Ok(conf)
    }
}

impl PeerDiscovery for Conf {
    fn peers_map(&self) -> &HashMap<HelperIdentity, peer::Config> {
        &self.peers_map
    }
}

// #[cfg(all(test, not(feature = "shuttle")))]
mod tests {
    use super::*;
    use crate::helpers::transport::http::discovery::PeerDiscovery;
    use crate::test_fixture::net::localhost_config_map;
    use hyper::Uri;

    const PUBLIC_KEY_1: &str = "13ccf4263cecbc30f50e6a8b9c8743943ddde62079580bc0b9019b05ba8fe924";
    const PUBLIC_KEY_2: &str = "925bf98243cf70b729de1d75bf4fe6be98a986608331db63902b82a1691dc13b";
    const PUBLIC_KEY_3: &str = "12c09881a1c7a92d1c70d9ea619d7ae0684b9cb45ecc207b98ef30ec2160a074";
    const URI_1: &str = "http://localhost:3000/";
    const URI_2: &str = "http://localhost:3001/";
    const URI_3: &str = "http://localhost:3002/";

    fn hex_str_to_public_key(hex_str: &str) -> x25519_dalek::PublicKey {
        let pk_bytes: [u8; 32] = hex::decode(hex_str)
            .expect("valid hex string")
            .try_into()
            .expect("hex should be exactly 32 bytes");
        pk_bytes.into()
    }

    #[test]
    fn parse_config() {
        let conf = localhost_config_map([3000, 3001, 3002]);

        let uri1 = URI_1.parse::<Uri>().unwrap();
        let id1 = HelperIdentity::from(uri1.clone());
        let value1 = conf.peers_map().get(&id1);
        assert!(value1.is_some(), "helper id {id1:?} not found");
        let value1 = value1.unwrap();
        assert_eq!(value1.origin, uri1);
        assert_eq!(value1.tls.public_key, hex_str_to_public_key(PUBLIC_KEY_1));

        let uri2 = URI_2.parse::<Uri>().unwrap();
        let id2 = HelperIdentity::from(uri2.clone());
        let value2 = conf.peers_map().get(&id2);
        assert!(value2.is_some(), "helper id {id2:?} not found");
        let value2 = value2.unwrap();
        assert_eq!(value2.origin, uri2);
        assert_eq!(value2.tls.public_key, hex_str_to_public_key(PUBLIC_KEY_2));

        let uri3 = URI_3.parse::<Uri>().unwrap();
        let id3 = HelperIdentity::from(uri3.clone());
        let value3 = conf.peers_map().get(&id3);
        assert!(value3.is_some(), "helper id {id3:?} not found");
        let value3 = value3.unwrap();
        assert_eq!(value3.origin, uri3);
        assert_eq!(value3.tls.public_key, hex_str_to_public_key(PUBLIC_KEY_3));
    }
}
