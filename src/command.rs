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
				SubCommand::with_name("getbalances")
					.about("Query balances of addresses"),

				SubCommand::with_name("syncextrinsics")
					.alias("syncxts")
					.about("Download and save extrinsics from remote node to local file through RPC. Alias `syncxts`")
					.arg(Arg::with_name("label_or_address")
						.help("The address or label")
						.required(false)
					),
				SubCommand::with_name("listextrinsics")
					.alias("listxts")
					.about("Print the list of extrinsics. Alias `listxts`")
					.args_from_usage("
						<label_or_address> 'The Address or label'
					"),
				SubCommand::with_name("setrpcurl")
					.about("Save RPC url")
					.args_from_usage("
						<url> 'RPC url, exmaple: wss://rpc.polkadot.io'
					"),
				SubCommand::with_name("watchaddress")
					.about("Add a watchonly address")
					.arg(Arg::with_name("addr")
						.help("The address")
						.required(true)
					)
					.arg(Arg::with_name("label")
						.help("The label")
						.required(false)
					),
				SubCommand::with_name("transfer")
					.about("Submit a transfer transaction")
					.arg(Arg::with_name("from")
						.help("The source address")
						.required(true)
					)
					.arg(Arg::with_name("to")
						.help("The destination address")
						.required(true)
					)
					.arg(Arg::with_name("amount")
						.help("Amount to be send")
						.required(true)
					),
			])
}