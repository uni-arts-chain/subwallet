
pub use sp_core::{
	crypto::{set_default_ss58_version, Ss58AddressFormat, Ss58Codec, Derive, AccountId32 },
	ed25519, sr25519, ecdsa, Pair, Public,
};

use blake2_rfc::blake2b::{ Blake2b, Blake2bResult };

pub trait Crypto: Sized {
	type Pair: Pair<Public = Self::Public>;
	type Public: Public + Ss58Codec + AsRef<[u8]> + std::hash::Hash;

	fn pair_from_secret_slice(slice: &[u8]) -> Result<Self::Pair, ()>;

	fn crypto_type() -> &'static str;

	fn to_address<P: Pair>(pair: &P) -> String;
}

pub struct Ed25519;

impl Crypto for Ed25519 {
	type Pair = ed25519::Pair;
	type Public = ed25519::Public;

	// `https://github.com/polkadot-js` ed25519's 64 bytes secret key is composed of 32 bytes secret part and 32 bytes public key
	// But in Substrate secret key must be bytes long, so we use the first 32 bytes of secret key
	fn pair_from_secret_slice(slice: &[u8]) -> Result<Self::Pair, ()> {
		match slice.len() {
			32 => {
				Self::Pair::from_seed_slice(slice).map_err(|_| () )
			},
			64 => {
				let mut secret_key = [0u8; 32];
				secret_key.copy_from_slice(&slice[..32]);
				Self::Pair::from_seed_slice(&secret_key[..]).map_err(|_| () )
			},
			_ => {
				Err(())
			},
		}
	}

	fn crypto_type() -> &'static str { "ed25519" }

	fn to_address<P: Pair>(pair: &P) -> String {
		pair.public().to_ss58check()
	}
}


pub struct Ecdsa;

impl Crypto for Ecdsa {
	type Pair = ecdsa::Pair;
	type Public = ecdsa::Public;

	fn pair_from_secret_slice(slice: &[u8]) -> Result<Self::Pair, ()> {
		if slice.len() != 32 {
			return Err(())
		}
		Self::Pair::from_seed_slice(slice).map_err(|_| () )
	}

	fn crypto_type() -> &'static str { "ecdsa" }

	fn to_address<P: Pair>(pair: &P) -> String {
		let raw = pair.public().to_raw_vec();
		let hash = Self::prehash(&raw[..]);
		let mut raw = [0u8; 32];
		raw.copy_from_slice(&hash.as_bytes()[..]);
		AccountId32::from(raw).to_ss58check()
	}
}

impl Ecdsa {
	fn prehash(data: &[u8]) -> Blake2bResult {
		let mut context = Blake2b::new(32);
		context.update(data);
		context.finalize()
	}
}


pub struct Sr25519;
impl Crypto for Sr25519 {
	type Pair = sr25519::Pair;
	type Public = sr25519::Public;

	fn pair_from_secret_slice(slice: &[u8]) -> Result<Self::Pair, ()> {
		// https://github.com/polkadot-js/wasm/blob/master/packages/wasm-crypto/src/sr25519.rs#L82
		// https://polkadot.js.org/apps expands `MiniSecretKey` to `SecretKey` with mode `ExpansionMode::Ed25519`
		// It is imcompatible with default implementation of Substrate `sp_core::sr25519`.
		// So we try to make key pair from `sr25519::Pair::from_seed_slice` first, then try `schnorrkel::SecretKey::from_ed25519_bytes`
		match Self::Pair::from_seed_slice(slice) {
			Ok(pair) => Ok(pair),
			Err(_) => {
				let sec = schnorrkel::SecretKey::from_ed25519_bytes(slice).map_err(|_| () )?;
				Ok(Self::Pair::from(sec))
			},
		}
	}

	fn crypto_type() -> &'static str { "sr25519" }

	fn to_address<P: Pair>(pair: &P) -> String {
		pair.public().to_ss58check()
	}
}


