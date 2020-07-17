mod error;
mod keystore;
mod crypto;
mod command;
mod wallet;
mod pkcs8;
mod networks;
mod rpc;
mod store;
mod sync;
mod transfer;
mod primitives;


use sp_core::crypto::{Ss58AddressFormat, set_default_ss58_version };
use std::path::{ Path, PathBuf };
use std::fs;
use crate::primitives::{ AccountId, AccountInfo };
use runtime::{ BalancesCall, Call };
use rust_decimal::prelude::*;
use std::ops::{Mul, Div};

use keystore::Keystore;
use crypto::*;
use wallet::*;
use rpc::*;
use networks::Network;
use store::*;

fn default_path() -> PathBuf {
	let mut path = dirs::home_dir().unwrap();
	path.push(".subwallet");
	if !path.exists() {
		fs::create_dir_all(path.clone()).expect("Failed to create default data path");
	}
	path
}

#[async_std::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
	let mut app = command::get_app();
	let matches = app.clone().get_matches();
	set_default_ss58_version(Ss58AddressFormat::PolkadotAccount);

	let data_path = default_path();
	let config_file = data_path.join("config.toml");
	let store = WalletStore::init(data_path.as_path().to_str());

	match matches.subcommand() {
		("getnewaddress", Some(matches)) => {
			let label = matches.value_of("label").unwrap();
			let mut address = if matches.is_present("ed25519") {
				Address::generate::<Ed25519>()
			} else if matches.is_present("ecdsa") {
				Address::generate::<Ecdsa>()
			} else {
				Address::generate::<Sr25519>()
			};

			address.label = label.to_string();
			store.save(address.clone());
			println!("{}", address.addr);
		}
		("listaddresses", Some(_)) => {
			let addresses = store.read_all();
			for address in addresses {
				address.print();
			}
		}
		("restore", Some(matches)) => {
			let file = matches.value_of("file").unwrap();

			let keystore = match Keystore::parse_from_file(file.to_string()) {
				Ok(keystore) => keystore,
				Err(_) => {
					println!("Failed to parse keystore file");
					return Ok(())
				}
			};

			let password = rpassword::read_password_from_tty(Some("Password: ")).ok();
			if let Ok(address) = Address::from_keystore(keystore, password) {
				store.save(address.clone());
				println!("{} is restored", address.addr);
			} else {
				println!("Failed to recover address");
				return Ok(())
			}
		}
		("backup", Some(matches)) => {
			let label  = matches.value_of("label").unwrap();
			let file  = matches.value_of("path").unwrap();

			let address = match store.read(label) {
				Some(address) => address,
				None => {
					println!("`{}` related address does not exist.", label);
					return Ok(())
				}
			};

			let path = Path::new(file);
			let full_path = if path.ends_with("/") || path.is_dir() { // dir
				let file_name = format!("{}.json", address.addr.as_str());
				let mut path = path.to_path_buf();
				path.push(file_name);
				path
			} else { // file
				path.to_path_buf()
			};

			if full_path.exists() {
				eprintln!("File `{}` aleady exists", full_path.to_str().unwrap());
				return Ok(())
			}

			let password = rpassword::read_password_from_tty(Some("Type password to encrypt seed: ")).ok();

			let keystore = address.into_keystore(password);

			if let Err(e) =  fs::write(full_path.clone(), keystore.to_json()) {
				println!("Failed to write to file: {:?}", e);
			} else {
				println!("Address `{}` is backed up to file `{}`", address.addr, full_path.to_str().unwrap());
			}
		},
		("transfer", Some(matches)) => {
			let from  = matches.value_of("from").unwrap();
			let to  = matches.value_of("to").unwrap();
			let amount  = matches.value_of("amount").unwrap();
			let value = Decimal::from_str(amount).map_err(|_err| "Invalid `amount`")?;
			let from_address = store.read(from).ok_or("`from` address does not exists")?;
			if from_address.is_watchonly() {
				let err = format!("Watchonly address `{}` can not do transfer", from_address.label);
				return Err(err.into());
			}

			let to_addr  = match store.read(to) {
				Some(v) => v.addr,
				None => to.to_string(),
			};


			let from_account_id = AccountId::from_ss58check(&from_address.addr).map_err(error::Error::PublicKey)?;
			let to_account_id = AccountId::from_ss58check(&to_addr).map_err(error::Error::PublicKey)?;

			let config = rpc::Config::parse_from_file(config_file.as_path())?;
			let url = config.get_url(Network::Polkadot).ok_or("rpc url is not set")?;
			let rpc = Rpc::new(url.clone()).await;
			let genesis_hash = rpc.genesis_hash().await?;
			let info: AccountInfo = rpc.get_account_info(from_account_id.clone()).await?;
			let properties = rpc.system_properties().await?;
			let decimals = properties["tokenDecimals"].as_u64().unwrap();
			let multipler: Decimal = 10u64.saturating_pow(decimals as u32).into();
			let amount = value.mul(multipler).to_u128().unwrap();

			let call = Call::Balances(BalancesCall::transfer(to_account_id, amount));
			let xt = match from_address.crypto_type.as_str() {
				"sr25519" => {
					let signer = from_address.into_pair::<Sr25519>();
					transfer::make_extrinsic::<Sr25519>(call, info.nonce, signer, genesis_hash)?
				},
				"ed25519" => { 
					let signer =  from_address.into_pair::<Ed25519>();
					transfer::make_extrinsic::<Ed25519>(call, info.nonce, signer, genesis_hash)?
				},
				"ecdsa" => { 
					let signer = from_address.into_pair::<Ecdsa>();
					transfer::make_extrinsic::<Ecdsa>(call, info.nonce, signer, genesis_hash)?
				},
				_ => unreachable!(),
			};
			let xt_hash = rpc.submit_extrinsic(xt).await?;
			println!("{:?}", xt_hash);
		},
		("getbalances", Some(_matches)) => {
			let addresses = store.read_all();
			let accounts = addresses.iter().map(|address| {
				AccountId::from_ss58check(&address.addr).unwrap()
			}).collect();

			let config = rpc::Config::parse_from_file(config_file.as_path())?;
			let url = config.get_url(Network::Polkadot).ok_or("rpc url is not set")?;
			let rpc = Rpc::new(url).await;
			let balances = rpc.get_balances(accounts).await?;
			let properties = rpc.system_properties().await?;
			let decimals = properties["tokenDecimals"].as_u64().unwrap();
			let divider: Decimal = 10u64.saturating_pow(decimals as u32).into();
			let unit = properties["tokenSymbol"].as_str().unwrap();
			for (addr, balance) in balances {
				let value = Decimal::from_str(balance.to_string().as_str()).unwrap().div(divider);
				println!("{:<55} {:>30} {}", addr, value, unit);
			}
		},
		("syncextrinsics", Some(matches)) => {
			let addresses: Vec<Address> = match matches.value_of("label_or_address") {
				Some(label) => {
					match store.read(label) {
						Some(address) => vec![address],
						None => {
							let err = format!("`{}` related address does not exist.", label);
							return Err(err.into());
						},
					}
				},
				None => store.read_all(),
			};

			let config = rpc::Config::parse_from_file(config_file.as_path())?;
			let url = config.get_url(Network::Polkadot).ok_or("rpc url is not set")?;

			let rpc = Rpc::new(url.clone()).await;
			let tip_header = rpc.header(None).await?.unwrap();
			let total_number = tip_header.number;
			let accounts: Vec<AccountId> = addresses.iter().map(|address| AccountId::from_ss58check(address.addr.as_str()).unwrap()).collect();

			sync::scan(url, total_number, accounts).await?;
		},
		("listextrinsics", Some(matches)) => {
			let label = matches.value_of("label_or_address").unwrap();
			let address = store.read(label).ok_or("The label or address does not exists")?;
			let xt_store = FileStore::get(&address.addr);
			let mut xts = xt_store.read_all();
			xts.sort_by(|a,b| b.block_number.partial_cmp(&a.block_number).unwrap());
			for xt in xts.iter() {
				xt.print();
			}
		},
		("setrpcurl", Some(matches)) => {
			let url  = matches.value_of("url").unwrap();
			let mut config = match rpc::Config::parse_from_file(config_file.as_path()) {
				Ok(config) => config,
				Err(_) => rpc::Config::new(),
			};
			let rpc = Rpc::new(url.to_string()).await;
			let properties = rpc.system_properties().await?;
			let network: Network = properties["ss58Format"].as_u64().unwrap().into();
			config.set_url(network, url.to_string());
			config.write_to_file(config_file.as_path())?;
			config.print();
		},
		("watchaddress", Some(matches)) => {
			let addr  = matches.value_of("addr").unwrap();
			let label  = matches.value_of("label").unwrap_or("");
			let _check = AccountId::from_ss58check(addr).map_err(|_err| "Invalid address" )?;

			let mut address = Address::default();
			address.label = label.to_string();
			address.addr = addr.to_string();
			store.save(address.clone());
			println!("{}", address.addr);
		},
		_ => {
			app.print_help().unwrap();
			println!();
		}
	}
	Ok(())
}