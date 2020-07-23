use std::fs;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use crate::crypto::*;
use crate::pkcs8;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Encoding {
  pub content: Vec<String>,
  pub r#type: String,
  pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Keystore {
  pub address: String,
  pub encoded: String,
  pub encoding: Encoding,
  pub meta: Value,
}


impl Keystore {
  pub fn parse_from_file(path: String) -> Result<Self, ()> {
    let data = fs::read_to_string(path).map_err(|_| () )?;
    let keystore: Self = serde_json::from_str(&data).map_err( |_| () )?;
    Ok(keystore)
  }

  pub fn crypto(&self) -> String {
    self.encoding.content[1].clone()
  }

  pub fn label(&self) -> String {
    self.meta["name"].as_str().unwrap_or("").to_string()
  }

  pub fn genesis_hash(&self) -> String {
    self.meta["genesisHash"].as_str().unwrap_or("").to_string()
  }

  pub fn when_created(&self) -> u64 {
    self.meta["whenCreated"].as_u64().unwrap_or(0u64)
  }

  pub fn to_json(&self) -> String {
    serde_json::to_string(&self).unwrap()
  }

  pub fn encoded_bytes(&self) -> Vec<u8> {
    let encoded = if self.encoded.starts_with("0x") {
      &self.encoded[2..]
    } else {
      &self.encoded
    };
    hex::decode(encoded).unwrap_or(vec![])
  }

  pub fn into_pair<T: Crypto>(&self, password: Option<String>) -> Result<T::Pair, ()> {
    let encoded = self.encoded_bytes();
    if encoded.is_empty() {
      return Err(())
    }
    match pkcs8::decode(&encoded[..], password) {
      Ok((_, secret_key)) => {
        T::pair_from_secret_slice(&secret_key[..])
      },
      Err(_) => Err(())
    }
  }
}


#[cfg(test)]
mod tests {
  use super::*;
  use hex_literal::hex;
  use sp_core::{ ed25519, sr25519, ecdsa, Pair };

  #[test]
  fn test_parse_invalid_json_file() {
    match Keystore::parse_from_file("tests/fixtures/invalid.json".into()) {
      Ok(_) => unreachable!(),
      Err(e) => assert_eq!(e, ())
    }
  }

  #[test]
  fn test_parse_valid_json_file() {
    match Keystore::parse_from_file("tests/fixtures/ecdsa.json".into()) {
      Ok(keystore) => {
        assert_eq!(keystore.address, "13SmLJEpENqt1mdZsFjhq8BgYYTBPAgPxrjaad4yNd4Bgw7Y");
        assert_eq!(keystore.encoding.content[1], "ecdsa");
      }
      Err(_) => unreachable!()
    }
  }


  #[test]
  fn test_into_pair_for_ecdsa() {
    let seed = hex!("bda7ce4ab5c0bdcfbf3f5353adb1ae795aa793261dd478c26cb97735b68bc687");
    let expect_pair = ecdsa::Pair::from_seed(&seed);
    
    let keystore = Keystore::parse_from_file("tests/fixtures/ecdsa.json".into()).unwrap();
    let password = Some("111111".to_string());
    let pair = keystore.into_pair::<Ecdsa>(password).unwrap();
    assert_eq!(pair.to_raw_vec(), expect_pair.to_raw_vec());
    assert_eq!(pair.public(), expect_pair.public());
  }

  #[test]
  fn test_into_pair_for_ed25519() {
    let seed = hex!("bda7ce4ab5c0bdcfbf3f5353adb1ae795aa793261dd478c26cb97735b68bc687");
    let expect_pair = ed25519::Pair::from_seed(&seed);

    let keystore = Keystore::parse_from_file("tests/fixtures/ed25519.json".into()).unwrap();
    let password = Some("111111".to_string());
    let pair = keystore.into_pair::<Ed25519>(password).unwrap();

    assert_eq!(pair.to_raw_vec(), expect_pair.to_raw_vec());
    assert_eq!(pair.public(), expect_pair.public());
  }

  #[test]
  fn test_into_pair_for_sr25519() {
    let seed = hex!("bda7ce4ab5c0bdcfbf3f5353adb1ae795aa793261dd478c26cb97735b68bc687");
    let expect_pair = sr25519::Pair::from_seed(&seed);

    let keystore = Keystore::parse_from_file("tests/fixtures/sr25519.json".into()).unwrap();
    let password = Some("111111".to_string());
    let pair = keystore.into_pair::<Sr25519>(password).unwrap();

    assert_eq!(pair.to_raw_vec(), expect_pair.to_raw_vec());
    assert_eq!(pair.public(), expect_pair.public());
  }

}



