
pub use sp_core::{
	crypto::{set_default_ss58_version, Ss58AddressFormat, Ss58Codec, Derive },
	ed25519, sr25519, ecdsa, Pair, Public,
};

use blake2_rfc::blake2b::{ Blake2b, Blake2bResult };

pub trait Crypto: Sized {
	type Pair: Pair<Public = Self::Public>;
	type Public: Public + Ss58Codec + AsRef<[u8]> + std::hash::Hash;

	fn pair_from_secret_slice(slice: &[u8]) -> Result<Self::Pair, ()>;

	fn crypto_type() -> &'static str;
}

pub struct Ed25519;

impl Crypto for Ed25519 {
	type Pair = ed25519::Pair;
	type Public = ed25519::Public;

	fn pair_from_secret_slice(slice: &[u8]) -> Result<Self::Pair, ()> {
		// if slice.len() != SEC_LENGTH  {
		// 	return Err(())
		// }
		let mut secret_key = [0u8; 32];
		secret_key.copy_from_slice(&slice[..32]);
		Ok(Self::Pair::from_seed_slice(&secret_key[..]).unwrap())
	}

	fn crypto_type() -> &'static str { "ed25519" }
}


pub struct Ecdsa;
struct EcdsaPublic([u8; 32]);
impl Derive for EcdsaPublic {}

impl Default for EcdsaPublic {
	fn default() -> Self {
		EcdsaPublic([0u8; 32])
	}
}

impl AsRef<[u8]> for EcdsaPublic {
	fn as_ref(&self) -> &[u8] {
		&self.0[..]
	}
}
impl AsMut<[u8]> for EcdsaPublic {
	fn as_mut(&mut self) -> &mut [u8] {
		&mut self.0[..]
	}
}

impl Crypto for Ecdsa {
	type Pair = ecdsa::Pair;
	type Public = ecdsa::Public;

	fn pair_from_secret_slice(slice: &[u8]) -> Result<Self::Pair, ()> {
		let mut secret_key = [0u8; 32];
		secret_key.copy_from_slice(&slice[..32]);
		let pair = Self::Pair::from_seed_slice(&secret_key[..]).unwrap();
		Ok(pair)
	}
	fn crypto_type() -> &'static str { "ecdsa" }
}

impl Ecdsa {
	// Return `https://github.com/polkadot-js` compatible address.
	pub fn to_js_ss58check<P: Pair>(pair: &P) -> String {
		let raw = pair.public().to_raw_vec();
		let hash = Self::prehash(&raw[..]);
		let mut raw = [0u8; 32];
		raw.copy_from_slice(&hash.as_bytes()[..]);
		let public = EcdsaPublic(raw);
		public.to_ss58check()
	}

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
		Ok(Self::Pair::from_seed_slice(&slice[..]).unwrap())
	}

	fn crypto_type() -> &'static str { "sr25519" }
}


