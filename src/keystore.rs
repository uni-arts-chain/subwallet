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





