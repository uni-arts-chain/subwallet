
pub use polkadot_primitives::v0::{
  Hash, 
  BlockNumber, 
  Nonce, 
  Balance, 
  AccountId, 
  Signature,
  AccountPublic,
};

pub type EventRecord = frame_system::EventRecord<runtime::Event, Hash>;
pub type AccountData = <runtime::Runtime as frame_system::Trait>::AccountData;
pub type AccountInfo = frame_system::AccountInfo<Nonce, AccountData>;
pub type Properties = serde_json::map::Map<String, serde_json::Value>;


pub const SCAN_STEP: u64 = 500;