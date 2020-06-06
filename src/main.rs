
mod keystore;
mod crypto;
mod command;
mod wallet;
mod pkcs8;
mod networks;

use sp_core::crypto::{Ss58AddressFormat, set_default_ss58_version};
use std::path::Path;
use std::fs;
use keystore::Keystore;
use crypto::*;
use wallet::*;

fn main() {
	let mut app = command::get_app();
	let matches = app.clone().get_matches();
	set_default_ss58_version(Ss58AddressFormat::PolkadotAccount);
	let store = WalletStore::init(None);

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
					return
				}
			};

			let password = rpassword::read_password_from_tty(Some("Password: ")).ok();
			if let Ok(address) = Address::from_keystore(keystore, password) {
				store.save(address.clone());
				println!("{} is restored", address.addr);
			} else {
				println!("Failed to recover address");
				return
			}
		}
		("backup", Some(matches)) => {
			let label  = matches.value_of("label").unwrap();
			let file  = matches.value_of("path").unwrap();

			let address = match store.read(label) {
				Some(address) => address,
				None => {
					println!("`{}` related address does not exist.", label);
					return
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
				return
			}

			let password = rpassword::read_password_from_tty(Some("Type password to encrypt seed: ")).ok();

			let keystore = address.into_keystore(password);

			if let Err(e) =  fs::write(full_path.clone(), keystore.to_json()) {
				println!("Failed to write to file: {:?}", e);
			} else {
				println!("Address `{}` is backed up to file `{}`", address.addr, full_path.to_str().unwrap());
			}
		}
		_ => {
			app.print_help().unwrap();
			println!();
		}
	}
}