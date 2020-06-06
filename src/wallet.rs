use serde::{Serialize, Deserialize};
use rustbreak::{FileDatabase};
use rustbreak::deser::Bincode;
use serde_json::json;

use std::path::PathBuf;
use std::time::SystemTime;
use std::fs;

use crate::keystore::{Keystore, Encoding};
use crate::crypto::*;
use crate::pkcs8;
use crate::networks::Network;

const DEFAULT_WALLET_NAME: &'static str = "polkadot";


#[derive(Eq, PartialEq, Debug, Serialize, Deserialize, Clone, Default)]
pub struct Address {
	pub addr: String,
	pub label: String,
	pub crypto_type: String,
	pub seed: Vec<u8>,
	pub network: String,
	pub created_at: u64,
}

impl Address {
	pub fn print(&self) {
		println!("{:<15} {:<55} {:<7}", self.label, self.addr, self.crypto_type);
	}

	pub fn into_keystore(&self, password: Option<String>) -> Keystore {
		let mut keystore = Keystore {
			address: self.addr.clone(),
			encoded: "".to_string(),
			encoding: Encoding {
				content: vec!["pkcs8".to_owned()],
				r#type: "xsalsa20-poly1305".to_owned(),
				version: "2".to_owned(),
			},
			meta: json!({
				"genesisHash": Network::default().genesis_hash(),
				"name": self.label,
				"tags": [],
				"whenCreated": self.created_at,
			}),
		};

		let (public_key, secret_key) = match self.crypto_type.as_str() {
			"sr25519" => {
				let pair = Sr25519::pair_from_secret_slice(&self.seed[..]).unwrap();
				(pair.public().to_raw_vec(), pair.to_raw_vec())
			},
			"ed25519" => {
				let pair = Ed25519::pair_from_secret_slice(&self.seed[..]).unwrap();
				(pair.public().to_raw_vec(), pair.to_raw_vec())
			},
			"ecdsa" => {
				let pair = Ecdsa::pair_from_secret_slice(&self.seed[..]).unwrap();
				// Use `https://polkadot.js.org/apps` compatible address format
				keystore.address = Ecdsa::to_js_ss58check(&pair);
				(pair.public().to_raw_vec(), pair.to_raw_vec())
			}
			_ => unreachable!()
		};

		let encoded = pkcs8::encode(&secret_key[..], &public_key[..], password).unwrap();
		keystore.encoded = format!("0x{}", hex::encode(encoded));
		keystore.encoding.content.push(self.crypto_type.clone());
		keystore
	}

	pub fn from_keystore(keystore: Keystore, password: Option<String>) -> Result<Self, ()> {
		let mut address = Self::default();
		address.label = keystore.label();
		address.created_at = keystore.when_created();
		address.crypto_type = keystore.crypto().clone();
		address.network = Network::from_genesis_hash(&keystore.genesis_hash()).into();

		match keystore.crypto().as_str() {
			"ecdsa" => {
				if let Ok(pair) = keystore.into_pair::<Ecdsa>(password) {
					address.addr = pair.public().to_ss58check();
					address.seed = pair.to_raw_vec();
				} else {
					return Err(())
				}
			},
			"sr25519" => {
				if let Ok(pair) = keystore.into_pair::<Sr25519>(password) {
					address.addr = pair.public().to_ss58check();
					address.seed = pair.to_raw_vec();
				} else {
					return Err(())
				}
			},
			"ed25519" => {
				if let Ok(pair) = keystore.into_pair::<Ed25519>(password) {
					address.addr = pair.public().to_ss58check();
					address.seed = pair.to_raw_vec();
				} else {
					return Err(())
				}
			}
			_ => {
				return Err(())
			}
		}

		Ok(address)
	}

	pub fn generate<T: Crypto>() -> Self {
		let (pair, _, seed) = T::Pair::generate_with_phrase(None);
		let seed_slice: &[u8] = seed.as_ref();
		let addr = pair.public().to_ss58check();
		let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64;
		Address {
			label: String::default(),
			addr: addr,
			crypto_type: T::crypto_type().to_owned(),
			network: Network::default().into(),
			seed: seed_slice.to_vec(),
			created_at: now,
		}
	}

}

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Wallet {
	pub name: String,
	pub address_book: Vec<Address>,
}

impl Wallet {
	pub fn new(name: String) -> Self {
		Self {
			name: name,
			address_book: vec![],
		}
	}

	pub fn add(&mut self, address: Address) {
		let addr = address.clone().addr;
		if self.get(addr.as_str()).is_none() {
			self.address_book.push(address);
		}
	}

	#[allow(dead_code)]
	pub fn delete(&mut self, label: &str) {
		self.address_book.retain(|address| address.label.as_str() != label );
	}

	pub fn get(&self, label_or_addr: &str) -> Option<&Address>{
		for address in &self.address_book {
			if address.label.as_str() == label_or_addr || address.addr.as_str() == label_or_addr {
				return Some(&address)
			}
		}
		return None;
	}
}

pub struct WalletStore(FileDatabase<Wallet, Bincode>);

impl WalletStore {
	pub fn init(path: Option<&str>) -> Self {
		let file = path.map(|v| {
			PathBuf::from(v)
		}).unwrap_or_else(|| {
			let mut file = dirs::home_dir().unwrap();
			file.push(".subwallet");
			file.push(DEFAULT_WALLET_NAME);
			file
		});

		if !file.exists() {
			fs::create_dir_all(file.parent().unwrap()).expect("Failed to create wallet file");
		}

		let backend = Wallet::new(DEFAULT_WALLET_NAME.to_owned());
		let db = FileDatabase::<Wallet, Bincode>::from_path(file, backend).expect("Failed to initialize file database.");
		Self(db)
	}

	pub fn load(&self) {
		let _ = self.0.load();
	}

	pub fn save(&self, address: Address) {
		self.load();
		self.0.write(|backend| {
			backend.add(address)
		}).expect("Failed to write address");
		self.0.save().expect("Failed to save");
	}

	pub fn read(&self, label: &str) -> Option<Address> {
		self.load();
		let backend = self.0.borrow_data().expect("Failed to read data");
		let v = backend.get(label);
		match v {
			Some(addr) => Some(addr.clone()),
			None => None
		}
	}

	pub fn read_all(&self) -> Vec<Address> {
		self.load();
		let backend = self.0.borrow_data().expect("Failed to read data");
		backend.address_book.clone()
	}
}

