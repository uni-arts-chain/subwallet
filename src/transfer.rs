
use runtime::{Call, Runtime, SignedPayload, UncheckedExtrinsic, VERSION, };
use crate::primitives::{Hash, Nonce as Index, Balance, Signature, AccountPublic };
use crate::crypto::{ Crypto, Pair, ed25519, sr25519, ecdsa, Ss58Codec };
use crate::error::Result;
use codec::{ Encode };
use sp_runtime::generic::Era;
use sp_runtime::traits::IdentifyAccount;

type SignatureOf<C> = <<C as Crypto>::Pair as Pair>::Signature;
type PublicOf<C> = <<C as Crypto>::Pair as Pair>::Public;

pub trait SignatureT: AsRef<[u8]> + AsMut<[u8]> + Default {
	/// Converts the signature into a runtime account signature, if possible. If not possible, bombs out.
	fn into_runtime(self) -> Signature {
		panic!("This cryptography isn't supported for this runtime.")
	}
}
pub trait PublicT: Sized + AsRef<[u8]> + Ss58Codec {
	/// Converts the public key into a runtime account public key, if possible. If not possible, bombs out.
	fn into_runtime(self) -> AccountPublic {
		panic!("This cryptography isn't supported for this runtime.")
	}
}
impl SignatureT for ed25519::Signature { fn into_runtime(self) -> Signature { self.into() } }
impl SignatureT for sr25519::Signature { fn into_runtime(self) -> Signature { self.into() } }
impl SignatureT for ecdsa::Signature { fn into_runtime(self) -> Signature { self.into() } }
impl PublicT for sr25519::Public { fn into_runtime(self) -> AccountPublic { self.into() } }
impl PublicT for ed25519::Public { fn into_runtime(self) -> AccountPublic { self.into() } }
impl PublicT for ecdsa::Public { fn into_runtime(self) -> AccountPublic { self.into() } }


pub fn make_extrinsic<C: Crypto>(
	function: Call,
	nonce: Index,
	signer: C::Pair,
	genesis_hash: Hash,
) -> Result<UncheckedExtrinsic> where 
	SignatureOf<C>: SignatureT,
	PublicOf<C>: PublicT,
{

	let extra = |i: Index, f: Balance| {
		(
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckMortality::<Runtime>::from(Era::Immortal),
			frame_system::CheckNonce::<Runtime>::from(i),
			frame_system::CheckWeight::<Runtime>::new(),
			transaction_payment::ChargeTransactionPayment::<Runtime>::from(f),
			runtime_common::registrar::LimitParathreadCommits::<Runtime>::new(),
			runtime_common::parachains::ValidateDoubleVoteReports::<Runtime>::new(),
			grandpa::ValidateEquivocationReport::<Runtime>::new(),
			runtime_common::claims::PrevalidateAttests::<Runtime>::new(),
		)
	};
	let raw_payload = SignedPayload::from_raw(
		function,
		extra(nonce, 0),
		(
			VERSION.spec_version,
			VERSION.transaction_version,
			genesis_hash,
			genesis_hash,
			(),
			(),
			(),
			(),
			(),
			(),
			(),
		),
	);
	let signature = raw_payload.using_encoded(|payload| signer.sign(payload)).into_runtime();
	let signer_account_id = signer.public().into_runtime().into_account().into();
	let (function, extra, _) = raw_payload.deconstruct();

	let xt = UncheckedExtrinsic::new_signed(
		function,
		signer_account_id,
		signature,
		extra,
	);
	Ok(xt)
}