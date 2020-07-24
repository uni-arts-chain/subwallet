
pub const POLKADOT_GENESIS_HASH: &'static str = "0x91b171bb158e2d3848fa23a9f1c25182fb8e20313b2c1eb49219da7a70ce90c3";
pub const KUSAMA_GENESIS_HASH: &'static str = "0xb0a8d493285c2df73290dfb7e61f870f17b41801197a149ca93654499ea3dafe";

#[derive(Clone, Eq, PartialEq, Debug, Copy)]
pub enum Network {
  Polkadot,
  Kusama,
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
      "kusama" => Network::Kusama,
      _ => Network::Unknow,
    }
  }
}

impl From<u8> for Network {
  fn from(v: u8) -> Network {
    match v {
      0 => Network::Polkadot,
      2 => Network::Kusama,
      _ => Network::Unknow,
    }
  }
}

impl From<u64> for Network {
  fn from(v: u64) -> Network {
    match v {
      0 => Network::Polkadot,
      2 => Network::Kusama,
      _ => Network::Unknow,
    }
  }
}

impl From<Network> for &'static str {
  fn from(n: Network) -> &'static str {
    match n {
      Network::Polkadot => "polkadot",
      Network::Kusama => "kusama",
      _ => "unknow",
    }
  }
}

impl From<Network> for String {
  fn from(n: Network) -> String {
    match n {
      Network::Polkadot => "polkadot".to_string(),
      Network::Kusama => "kusama".to_string(),
      _ => "unknow".to_string(),
    }
  }
}

impl Network {
  pub fn genesis_hash(&self) -> &'static str {
    match self {
      Network::Polkadot => POLKADOT_GENESIS_HASH,
      Network::Kusama => KUSAMA_GENESIS_HASH,
      _ => "",
    }
  }

  pub fn from_genesis_hash(hash: &str) -> Self {
    match hash {
      POLKADOT_GENESIS_HASH => Network::Polkadot,
      KUSAMA_GENESIS_HASH => Network::Kusama,
      _ => Network::Unknow,
    }
  }
}
