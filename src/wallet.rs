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
		if self.seed.len() == 0 {
			println!("{:<15} {:<55} {:<7}", self.label, self.addr, "*");
		} else {
			println!("{:<15} {:<55} {:<7}", self.label, self.addr, self.crypto_type);
		}
	}

	pub fn is_watchonly(&self) -> bool {
		self.seed.len() == 0
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
					address.addr = Ecdsa::to_address(&pair);
					address.seed = pair.to_raw_vec();
				} else {
					return Err(())
				}
			},
			"sr25519" => {
				if let Ok(pair) = keystore.into_pair::<Sr25519>(password) {
					address.addr = Sr25519::to_address(&pair);
					address.seed = pair.to_raw_vec();
				} else {
					return Err(())
				}
			},
			"ed25519" => {
				if let Ok(pair) = keystore.into_pair::<Ed25519>(password) {
					address.addr = Ed25519::to_address(&pair);
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
		let addr = T::to_address(&pair);
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

	pub fn into_pair<T: Crypto>(&self) -> <T as Crypto>::Pair {
		T::pair_from_secret_slice(&self.seed[..]).unwrap()
	}
}

#[cfg(test)]
mod address_tests {
	use hex_literal::hex;
	use sp_core::{ 
		crypto::{Ss58AddressFormat, set_default_ss58_version},
		ed25519, sr25519, ecdsa, Pair
	};
	use super::Address;
	use crate::keystore::Keystore;
	use crate::crypto::*;

	fn setup() {
		set_default_ss58_version(Ss58AddressFormat::PolkadotAccount);
	}

	#[test]
	fn generate_should_works() {
		let ed_address = Address::generate::<Ed25519>();
		assert_ne!(ed_address.seed, Vec::<u8>::new());

		let sr_address = Address::generate::<Sr25519>();
		assert_ne!(sr_address.seed, Vec::<u8>::new());

		let ec_address = Address::generate::<Ecdsa>();
		assert_ne!(ec_address.seed, Vec::<u8>::new());
	}

	#[test]
	fn test_from_keystore_with_incorrect_password() {
		setup();
		let keystore = Keystore::parse_from_file("tests/fixtures/ecdsa.json".into()).unwrap();
		let password = Some("incorrect".to_owned()); // 111111 is correct password
		match Address::from_keystore(keystore, password) {
			Ok(_) => unreachable!(),
			Err(e) => assert_eq!(e, ()),
		}
	}


	#[test]
	fn test_from_keystore_for_ecdsa() {
		setup();

		let keystore = Keystore::parse_from_file("tests/fixtures/ecdsa.json".into()).unwrap();
		let password = Some("111111".to_string());

		let seed = hex!("bda7ce4ab5c0bdcfbf3f5353adb1ae795aa793261dd478c26cb97735b68bc687");
		let pair = ecdsa::Pair::from_seed(&seed);

		let expect_address = Address {
			addr: "13SmLJEpENqt1mdZsFjhq8BgYYTBPAgPxrjaad4yNd4Bgw7Y".to_owned(),
			label: "ecdsa".to_owned(),
			crypto_type: "ecdsa".to_owned(),
			seed: pair.to_raw_vec(),
			network: "polkadot".to_owned(),
			created_at: 1591600236132u64,
		};

		let address = Address::from_keystore(keystore, password).unwrap();

		assert_eq!(address, expect_address);
	}

	#[test]
	fn test_from_keystore_for_sr25519() {
		setup();

		let keystore = Keystore::parse_from_file("tests/fixtures/sr25519.json".into()).unwrap();
		let password = Some("111111".to_string());

		let seed = hex!("bda7ce4ab5c0bdcfbf3f5353adb1ae795aa793261dd478c26cb97735b68bc687");
		let pair = sr25519::Pair::from_seed(&seed);

		let expect_address = Address {
			addr: "14cwHq7pwagFBTdT9E3TTzh2WsuugSAoxL53fpywct2KVSQG".to_owned(),
			label: "sr25519".to_owned(),
			crypto_type: "sr25519".to_owned(),
			seed: pair.to_raw_vec(),
			network: "polkadot".to_owned(),
			created_at: 1591600865993u64,
		};
		let address = Address::from_keystore(keystore, password).unwrap();
		assert_eq!(address, expect_address);
	}

		#[test]
	fn test_from_keystore_for_ed25519() {
		setup();

		let keystore = Keystore::parse_from_file("tests/fixtures/ed25519.json".into()).unwrap();
		let password = Some("111111".to_string());

		let seed = hex!("bda7ce4ab5c0bdcfbf3f5353adb1ae795aa793261dd478c26cb97735b68bc687");
		let pair = ed25519::Pair::from_seed(&seed);

		let expect_address = Address {
			addr: "14TouV8puYdaN72wMvNirvZsvcvYk5GRfTwJ7XF4P9fibL3m".to_owned(),
			label: "ed25519".to_owned(),
			crypto_type: "ed25519".to_owned(),
			seed: pair.to_raw_vec(),
			network: "polkadot".to_owned(),
			created_at: 1591600763959u64,
		};
		let address = Address::from_keystore(keystore, password).unwrap();
		assert_eq!(address, expect_address);
	}

	#[test]
	fn test_into_keystore_for_ecdsa() {
		setup();

		let seed = hex!("bda7ce4ab5c0bdcfbf3f5353adb1ae795aa793261dd478c26cb97735b68bc687");
		let pair = ecdsa::Pair::from_seed(&seed);

		let address = Address {
			addr: Ecdsa::to_address(&pair),
			label: "ecdsa".to_owned(),
			crypto_type: "ecdsa".to_owned(),
			seed: pair.to_raw_vec(),
			network: "polkadot".to_owned(),
			created_at: 1591600236132u64,
		};
		let password = Some("111111".to_owned());
		let keystore = address.into_keystore(password.clone());

		let decoded_address = Address::from_keystore(keystore, password).unwrap();
		assert_eq!(address, decoded_address);
	}


	#[test]
	fn test_into_keystore_for_sr25519() {
		setup();

		let seed = hex!("bda7ce4ab5c0bdcfbf3f5353adb1ae795aa793261dd478c26cb97735b68bc687");
		let pair = sr25519::Pair::from_seed(&seed);

		let address = Address {
			addr: pair.public().to_ss58check(),
			label: "sr25519".to_owned(),
			crypto_type: "sr25519".to_owned(),
			seed: pair.to_raw_vec(),
			network: "polkadot".to_owned(),
			created_at: 1591600236132u64,
		};
		let password = Some("111111".to_owned());
		let keystore = address.into_keystore(password.clone());

		let decoded_address = Address::from_keystore(keystore, password).unwrap();
		assert_eq!(address, decoded_address);
	}

	#[test]
	fn test_into_keystore_for_ed25519() {
		setup();

		let seed = hex!("bda7ce4ab5c0bdcfbf3f5353adb1ae795aa793261dd478c26cb97735b68bc687");
		let pair = ed25519::Pair::from_seed(&seed);

		let address = Address {
			addr: pair.public().to_ss58check(),
			label: "ed25519".to_owned(),
			crypto_type: "ed25519".to_owned(),
			seed: pair.to_raw_vec(),
			network: "polkadot".to_owned(),
			created_at: 1591600236132u64,
		};
		let password = Some("111111".to_owned());
		let keystore = address.into_keystore(password.clone());

		let decoded_address = Address::from_keystore(keystore, password).unwrap();
		assert_eq!(address, decoded_address);
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
			let mut file = PathBuf::from(v);
			file.push(DEFAULT_WALLET_NAME);
			file
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

