use clap::{App, SubCommand, Arg};

pub fn get_app<'a, 'b>() -> App<'a, 'b> {
	App::new("subwallet")
 			.author("yxf <yxf4559@gmail.com>")
			.about("A simple Command Line Interface wallet for Polkadot/Substrate.")
			.version(env!("CARGO_PKG_VERSION"))
			.subcommands(vec![
				SubCommand::with_name("getnewaddress")
					.about("Generate a new address associated with label, deafult cryptography is sr25519")
					.arg(Arg::with_name("label")
						.help("The label name for the address to be linked to.")
						.required(true)
					).args_from_usage("
						-e, --ed25519 'Use Ed25519/BIP39 cryptography'
						-k, --ecdsa   'Use SECP256k1/ECDSA/BIP39 cryptography'
						-s, --sr25519 'Use Schnorr/Ristretto x25519/BIP39 cryptography'
					"),
				SubCommand::with_name("listaddresses")
					.about("Prints the list of addresses"),

				SubCommand::with_name("restore")
					.about("Restore address from json file")
					.args_from_usage("
						<file>  'The filename with path'
					"),
				SubCommand::with_name("backup")
					.about("Backup specified address to local json file")
					.args_from_usage("
						<label>  'Address or label to backup'
						<path>  'The destination directory or file'
					"),
			])
}