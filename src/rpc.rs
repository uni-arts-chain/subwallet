
use codec::{
  Decode,
  Encode,
};

use jsonrpsee::{
  common::{
    to_value as to_json_value,
    Params,
  },
  Client,
};

use frame_metadata::{ 
  RuntimeMetadataPrefixed,
  RuntimeMetadata,
};

use sp_core::{
  storage::{
    StorageChangeSet,
    StorageData,
    StorageKey,
  },
  twox_128,
  blake2_128,
  Bytes,
};

use sp_rpc::{
  list::ListOrValue
};


use runtime::{ SignedBlock, Header };
use toml::{ Value as TomlValue, value::Table };

use crate::networks::Network;
use crate::error::{Result};
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use crate::primitives::{
  Hash, 
  BlockNumber,
  Balance,
  AccountInfo,
  Properties,
  AccountId,
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Config {
  #[serde(rename = "rpc")]
  values: TomlValue,
}

impl Config {

  pub fn new() -> Self {
    Self {
      values: TomlValue::Table(Table::new()),
    }
  }

  pub fn parse_from_file(path: &Path) -> Result<Self> {
    if !path.exists() {
      return Err("rpc url is not set".into());
    }
    let data: String = fs::read_to_string(path)?;
    let config: Self = toml::from_str(&data)?;
    Ok(config)
  }

  pub fn get_url(&self, network: Network) -> Option<String> {
    let table = self.values.as_table()?;
    let url = table.get(network.into())?.as_str()?;
    Some(url.into())
  }

  pub fn set_url(&mut self, network: Network, url: String) {
    let table = self.values.as_table_mut().unwrap();
    table.insert(network.into(), url.into());
  }

  pub fn write_to_file(&self, path: &Path) -> Result<()> {
    let data = self.to_string()?;
    fs::write(path, data)?;
    Ok(())
  }

  pub fn to_string(&self) -> Result<String> {
    let val = toml::to_string(self)?;
    Ok(val)
  }

  pub fn print(&self){
    let table = self.values.as_table();
    match table {
      Some(table) => {
        for (k, v) in table {
          println!("{} = {}", k, v.as_str().unwrap());
        }
      },
      None => {
        println!("{:?}", "No data");
      }
    }
  }
}
#[derive(Clone)]
pub struct Rpc {
  client: Client
}

impl Rpc {

  pub async fn new(url: String) -> Self {
    let client = if url.starts_with("ws://") || url.starts_with("wss://") {
        // jsonrpsee::ws_client(&url).await.unwrap()
        crate::ws_client::create(&url)
    } else {
        jsonrpsee::http_client(&url)
    };
    Rpc{
      client
    }
  }

  /// Request system properties
  pub async fn system_properties(&self) -> Result<Properties> {
    match self.client.request("system_properties", Params::None).await {
      Ok(v) => Ok(v),
      Err(err) => Err(format!("{:?}", err).into())
    }
  }

  /// Submit extrinsic
  pub async fn submit_extrinsic<E: Encode>(
    &self,
    extrinsic: E,
  ) -> Result<Hash> {
    let bytes: Bytes = extrinsic.encode().into();
    let params = Params::Array(vec![to_json_value(bytes)?]);
    let xt_hash = self.client.request("author_submitExtrinsic", params).await?;
    Ok(xt_hash)
  }

  /// Request the block hash by block number
  pub async fn block_hash(
    &self,
    block_number: Option<BlockNumber>,
  ) -> Result<Option<Hash>> {
    let block_number = block_number.map(ListOrValue::Value);
    let params = Params::Array(vec![to_json_value(block_number)?]);
    let list_or_value  = self.client.request("chain_getBlockHash", params).await?;
    match list_or_value {
        ListOrValue::Value(hash) => Ok(hash),
        ListOrValue::List(_) => Err("Expected a Value, got a List".into()),
    }
  }

  /// Request genesis hash
  pub async fn genesis_hash(&self) -> Result<Hash> {
    self.block_hash(Some(0)).await.map(|hash| hash.unwrap())
  }

  /// Request the metadata
  #[allow(dead_code)]
  pub async fn metadata(&self, hash: Option<Hash>) -> Result<RuntimeMetadata> {
    let params = Params::Array(vec![to_json_value(hash)?]);
    let bytes: Bytes = self.client.request("state_getMetadata", params).await?;
    let meta: RuntimeMetadataPrefixed = Decode::decode(&mut &bytes[..])?;
    Ok(meta.1)
  }

  /// Requeset a block, returns latest block by default
  pub async fn block(&self, hash: Option<Hash>) -> Result<Option<SignedBlock>> {
    let params = Params::Array(vec![to_json_value(hash)?]);
    let block = self.client.request("chain_getBlock", params).await?;
    Ok(block)
  }

  /// Request a block header
  pub async fn header(&self, hash: Option<Hash>) -> Result<Option<Header>> {
    let params = Params::Array(vec![to_json_value(hash)?]);
    let block = self.client.request("chain_getHeader", params).await?;
    Ok(block)
  }

  /// Query storage
  pub async fn query_storage(
    &self, 
    keys: Vec<StorageKey>,
    from: Hash, 
    to: Option<Hash>) -> Result<Vec<StorageChangeSet<Hash>>> 
  {
    let params = Params::Array(vec![
      to_json_value(keys)?,
      to_json_value(from)?,
      to_json_value(to)?,
    ]);
    self.client
        .request("state_queryStorage", params)
        .await
        .map_err(Into::into)
  }

  /// Query storage at specific block, default is latest block hash
  pub async fn query_storage_at(
    &self, 
    keys: Vec<StorageKey>,
    at: Option<Hash>) -> Result<Vec<StorageChangeSet<Hash>>> 
  {
    let params = Params::Array(vec![
      to_json_value(keys)?,
      to_json_value(at)?,
    ]);
    self.client
        .request("state_queryStorageAt", params)
        .await
        .map_err(Into::into)
  }

  /// Get storage at specific block
  pub async fn get_storage(
    &self,
    key: StorageKey,
    at: Option<Hash>) -> Result<Option<StorageData>>
  {
    let params = Params::Array(vec![
      to_json_value(key)?,
      to_json_value(at)?,
    ]);
    self.client
        .request("state_getStorage", params)
        .await
        .map_err(Into::into)
  }

  /// Get balances of addresses
  pub async fn get_balances(&self, accounts: Vec<AccountId>) -> Result<Vec<(AccountId, Balance)>> 
  {
    let keys = accounts.iter().map(|id| {
      let mut key = twox_128(b"System").to_vec();
      key.extend(twox_128(b"Account").to_vec());
      key.extend(
        id.using_encoded(|v| {
          let mut r = blake2_128(v).to_vec();
          r.extend_from_slice(v);
          r
        })
      );
      StorageKey(key)
    }).collect();

    let sets: Vec<StorageChangeSet<Hash>> = self.query_storage_at(keys, None).await?;

    let balances: Vec<Balance> = sets[0].changes.iter().map(|(_, data)| {
      let info = data.clone().map_or(Default::default(), |v| {
        let input = v.0;
        AccountInfo::decode(&mut &input[..]).unwrap_or(Default::default())
      });
      info.data.free
    }).collect();
    let results: Vec<(AccountId, Balance)> = accounts.iter().zip(balances.iter()).map(|(k,v)| (k.clone(), v.clone())).collect();
    Ok(results)
  }

  #[allow(dead_code)]
  pub async fn get_account_info(&self, account: AccountId) -> Result<AccountInfo> {
    let mut key = twox_128(b"System").to_vec();
    key.extend(twox_128(b"Account").to_vec());
    key.extend(
      account.using_encoded(|v| {
        let mut r = blake2_128(v).to_vec();
        r.extend_from_slice(v);
        r
      })
    );

    let data: Option<StorageData> = self.get_storage(StorageKey(key), None).await?;

    let info = match data {
      Some(v) => {
        let input = v.0;
        AccountInfo::decode(&mut &input[..]).unwrap_or(Default::default())
      },
      None => Default::default(),
    };

    Ok(info)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use sp_core::crypto::{ Ss58Codec, Ss58AddressFormat, set_default_ss58_version };
  use runtime::{ BalancesCall, Call };
  use crate::crypto::Ed25519;
  use crate::wallet::Address;
  use crate::primitives::{ AccountId };
  use std::str::FromStr;

  #[test]
  fn test_config_parse_from_file() {
    let config = Config::parse_from_file("tests/fixtures/rpc_config.toml".as_ref()).unwrap();
    assert!(config.values.is_table());
  }
  #[test]
  fn test_config_get_url() {
    let config = Config::parse_from_file("tests/fixtures/rpc_config.toml".as_ref()).unwrap();
    let url = config.get_url(Network::Polkadot).unwrap();
    assert_eq!(url, "wss://rpc.polkadot.io".to_string())
  }

  #[test]
  fn test_config_set_url() {
    let mut config = Config::new();
    config.set_url(Network::Polkadot, "xxxx".to_string());
    assert_eq!(config.get_url(Network::Polkadot), Some("xxxx".to_string()));
  }

  #[test]
  fn test_config_to_string() {
    let config = Config::parse_from_file("tests/fixtures/rpc_config.toml".as_ref()).unwrap();
    let expect = fs::read_to_string("tests/fixtures/rpc_config.toml").unwrap();
    assert_eq!(expect, config.to_string().unwrap());
  }
  #[test]
  fn test_config_write_to_file() {
    let config = Config::parse_from_file("tests/fixtures/rpc_config.toml".as_ref()).unwrap();
    assert!(config.write_to_file("/tmp/rpc_config.toml".as_ref()).is_ok());
    let actual = Config::parse_from_file("/tmp/rpc_config.toml".as_ref()).unwrap();
    assert_eq!(config, actual)
  }
  

  async fn setup_rpc() -> Rpc {
    set_default_ss58_version(Ss58AddressFormat::PolkadotAccount);
    Rpc::new("wss://rpc.polkadot.io".into()).await
  }

  #[tokio::test]
  async fn test_metadata() {
    let rpc = setup_rpc().await;
    // let hash = rpc.block_hash(Some(212894)).await.unwrap();
    let meta = rpc.metadata(None).await;
    assert!(meta.is_ok());
  }

  #[tokio::test]
  async fn test_block_hash() {
    let rpc = setup_rpc().await;
    let hash = rpc.block_hash(Some(0)).await;
    match hash {
      Ok(v) => assert!(true, v),
      Err(_) => unreachable!(),
    }
  }

  #[tokio::test]
  async fn test_block() {
    let rpc = setup_rpc().await;
    let hash = rpc.block_hash(Some(0)).await.unwrap();
    let block = rpc.block(hash).await;
    match block {
      Ok(Some(block)) => assert_eq!(block.block.header.number, 0),
      _ => unreachable!()
    }
  }

  #[tokio::test]
  async fn test_query_storage() {
    let rpc = setup_rpc().await;
    let from = rpc.block_hash(None).await.unwrap().unwrap();
    let to = rpc.block_hash(None).await.unwrap();
    let mut key = sp_core::twox_128(b"System").to_vec();
    key.extend(sp_core::twox_128(b"Account").to_vec());
    let account_id = AccountId::from_ss58check("1Zb1gY6xf1pzNhsYgbbrpnVSmtv6J8Gz44kS9334BLDxJan").unwrap();

    let account_key = account_id.encode();
    let mut v = blake2_128(account_key.as_slice()).to_vec();
    v.extend_from_slice(account_key.as_slice());
    key.extend(v);

    let keys = vec![sp_core::storage::StorageKey(key)];
    let storage = rpc.query_storage(keys, from, to).await.unwrap();

    assert!(storage.len() > 0);
    let (_key, data) = storage[0].changes[0].clone();
    let input = data.unwrap().0.clone();

    let info = AccountInfo::decode(&mut &input[..]);
    assert!(info.is_ok());
  }

  #[tokio::test]
  async fn test_get_storage() {
    set_default_ss58_version(Ss58AddressFormat::PolkadotAccount);
    let rpc = setup_rpc().await;
    let block_hash = rpc.block_hash(None).await.unwrap();

    let mut key = sp_core::twox_128(b"System").to_vec();
    key.extend(sp_core::twox_128(b"Account").to_vec());
    let account_id = AccountId::from_ss58check("1Zb1gY6xf1pzNhsYgbbrpnVSmtv6J8Gz44kS9334BLDxJan").unwrap();

    let account_key = account_id.encode();
    let mut v = blake2_128(account_key.as_slice()).to_vec();
    v.extend_from_slice(account_key.as_slice());
    key.extend(v);

    let k = sp_core::storage::StorageKey(key);
    let storage = rpc.get_storage(k, block_hash).await.unwrap();
    assert!(storage.is_some());
  }

  #[tokio::test]
  async fn test_get_balances() {
    let rpc = setup_rpc().await;
    let accounts = vec![
      AccountId::from_ss58check("1Zb1gY6xf1pzNhsYgbbrpnVSmtv6J8Gz44kS9334BLDxJan").unwrap(),
      AccountId::from_ss58check("13mmmB4jM9H7Ad3c6Hk5kDawDi2aXRsQ6eDVCbQqDJZ9khAH").unwrap(),
    ];
    let balances = rpc.get_balances(accounts).await.unwrap();
    assert!(balances.len() > 0);
  }

  #[tokio::test]
  async fn test_get_account_info() {
    let rpc = setup_rpc().await;
    let id = AccountId::from_ss58check("1Qobp4G1snJPNWPz3onWpDVJGXtipBeF2EdLEdXT9aRRENe").unwrap();

    let info = rpc.get_account_info(id).await.unwrap();
    assert!(info.refcount > 0);
  }

  #[tokio::test]
  async fn submit_extrinsic_should_fail_since_transfer_is_disabled() {
    let rpc = setup_rpc().await;
    // let meta = rpc.metadata(None).await;
    let from_address = Address::generate::<Ed25519>();
    let to_address = Address::generate::<Ed25519>();
    let to_account_id = AccountId::from_ss58check(&to_address.addr).unwrap();
    let call = Call::Balances(BalancesCall::transfer(to_account_id, 100));
    let signer = from_address.into_pair::<Ed25519>();
    let genesis_hash = crate::networks::POLKADOT_GENESIS_HASH;
    let genesis_hash = Hash::from_str(&genesis_hash[2..]).unwrap();
    let xt = crate::transfer::make_extrinsic::<Ed25519>(call, 0, signer, genesis_hash).unwrap();
    let result = rpc.submit_extrinsic(xt).await;
    assert!(result.is_err());
  }
}