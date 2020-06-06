
pub const POLKADOT_GENESIS_HASH: &'static str = "0x91b171bb158e2d3848fa23a9f1c25182fb8e20313b2c1eb49219da7a70ce90c3";

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Network {
	Polkadot,
	Unknow,
}

impl Default for Network {
	fn default() -> Self {
		Network::Polkadot
	}
}

impl From<&str> for Network {
	fn from(name: &str) -> Network {
		match name {
			"polkadot" => Network::Polkadot,
			_ => Network::Unknow,
		}
	}
}

impl From<Network> for &'static str {
	fn from(n: Network) -> &'static str {
		match n {
			Network::Polkadot => "polkadot",
			_ => "unknow",
		}
	}
}

impl From<Network> for String {
	fn from(n: Network) -> String {
		match n {
			Network::Polkadot => "polkadot".to_string(),
			_ => "unknow".to_string(),
		}
	}
}

impl Network {
	pub fn genesis_hash(&self) -> &'static str {
		match self {
			Network::Polkadot => POLKADOT_GENESIS_HASH,
			_ => "",
		}
	}

	pub fn from_genesis_hash(hash: &str) -> Self {
		match hash {
			POLKADOT_GENESIS_HASH => Network::Polkadot,
			_ => Network::Unknow,
		}
	}
}
