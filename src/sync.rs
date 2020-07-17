
use futures::channel::mpsc::{ UnboundedSender, unbounded };
use futures::executor::ThreadPool;
use futures::join;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::{thread, time};

use codec::{ Decode, Compact };
use indicatif::{ ProgressBar, ProgressStyle };

use crate::primitives::{AccountId, EventRecord, Message, SCAN_STEP };
use runtime::{ Event };
use frame_system::{ Phase, RawEvent };
use sp_core::crypto::{ Ss58Codec };

use crate::rpc::*;
use crate::error::Result;
use crate::store::*;
use frame_support::traits::{GetCallMetadata};
// use frame_support::dispatch::{Callable, CallableCallFor};

pub async fn scan(url: String, tip_number: u32, accounts: Vec<AccountId>) -> Result<()> {
	let pool = ThreadPool::new()?;
	
	let threads_size = 1u8;
	let cursors: Vec<u32> = accounts.iter().map(|id| FileStore::get(id.to_ss58check().as_str()).read().scanned_at).collect();
	let start_number = cursors.iter().min().unwrap_or(&0u32).clone();
	let cursor = Arc::new(AtomicU64::new(start_number as u64));

	println!("Starting scan from height {:?} on {}", cursor, url.clone());
	let (tx, mut rx) = unbounded::<Message>();
	for _i in 0..threads_size {
		pool.spawn_ok(
			scan_task(
				url.clone(), 
				tx.clone(), 
				accounts.clone(), 
				cursor.clone(), 
				SCAN_STEP, 
				tip_number as u64
			)
		);
	}
	drop(tx);

	let rpc = Arc::new(Rpc::new(url.to_string()).await);
	let bar_length = tip_number - start_number;
	let bar = ProgressBar::new(bar_length as u64);
	bar.set_style(ProgressStyle::default_bar()
  								.template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7}")
  								.progress_chars("#>-"));
	loop {
		if let Ok(recv) = rx.try_next() {
			if recv.is_some() {
				process(rpc.clone(), recv.unwrap()).await;
			} else {
				bar.finish();
				for account in accounts.iter() {
					let addr = account.to_ss58check();
					let store = FileStore::get(addr.as_str());
					store.update(tip_number);
				}
				break;
			}
		} else {
			let interval = time::Duration::from_millis(5);
			thread::sleep(interval);
		}
		let current = cursor.load(Ordering::SeqCst);
		let pos = current - start_number as u64;
		if pos > bar.position() {
			bar.set_position(pos);
		}
	};

	Ok(())
}


async fn process(rpc: Arc<Rpc>, msg: Message) {
	let (block_hash, targets) = msg;
	let block = match rpc.block(Some(block_hash)).await {
		Ok(signed) => signed.unwrap().block,
		Err(err) => {
			println!("err = {:?}", err);
			return
		},
	};

	let number = block.header.number;
	let xts = block.extrinsics;

	for target in targets.into_iter() {
		let (accounts, record) = target;

		let index: usize = match record.phase {
			Phase::ApplyExtrinsic(i) => i as usize,
			_ => return,
		};

		let status: String = match record.event {
			Event::system(RawEvent::ExtrinsicFailed(..)) => "failed".to_string(),
			_ => "success".to_string(),
		};
		let xt = &xts[index];
		let xt_string = format!("{:?}", xt);
		let event_string = format!("{:?}", record.event);
		let data = xt.function.get_call_metadata();
		let mut signer: Option<String> = None;
		if let Some((address, _, _)) = &xt.signature {
			signer = Some(address.to_ss58check());
		}

		for account in accounts.iter() {
			let account_string = format!("{:?}", account);
			// extrinsic or event that contains account
			if xt_string.as_str().contains(account_string.as_str()) || event_string.as_str().contains(account_string.as_str()) {
				let extrinsic = Extrinsic {
					block_number: number,
					block_hash: format!("{:#x}", block_hash),
					index: index as u32,
					signer: signer.clone(),
					status: status.clone(),
					module: data.pallet_name.to_string(),
					call: data.function_name.to_string(),
				};
				let addr = account.clone().to_ss58check();
				let store = FileStore::get(addr.as_str());
				store.save(extrinsic);
			}
		}
	}
}

async fn scan_task(
	url: String, 
	tx: UnboundedSender<Message>, 
	accounts: Vec<AccountId>,
	cursor: Arc<AtomicU64>,
	step: u64,
	tip_number: u64)
{
	let rpc = Rpc::new(url).await;
	loop {
		let now = cursor.load(Ordering::SeqCst);
		if now > tip_number as u64 {
			break;
		}
		let p = cursor.fetch_add(step, Ordering::SeqCst);
		let start: u32 = p as u32;
		let end: u32 = p as u32 + step as u32 - 1;

		let (start_hash, end_hash) = join!(rpc.block_hash(Some(start)), rpc.block_hash(Some(end)));

  	let mut key = sp_core::twox_128(b"System").to_vec();
  	key.extend(sp_core::twox_128(b"Events").to_vec());
		let keys = vec![sp_core::storage::StorageKey(key)];
		let storage = rpc.query_storage(keys, start_hash.unwrap().unwrap(), end_hash.unwrap()).await.unwrap();

		for changeset in storage {
			let (_k, data) = changeset.changes[0].clone();
			if let Some(v) = data {
				let mut input = v.0.as_slice();
				let compact_len = <Compact<u32>>::decode(&mut input).unwrap();
    		let len = compact_len.0 as usize;

    		let mut targets = Vec::new();
    		for _i in 0..len {
    			let record = EventRecord::decode(&mut input).unwrap();
   				let event_string = format!("{:?}", record.clone().event);

   				let maybe: bool = match record.event {
						Event::system(RawEvent::ExtrinsicFailed(..)) => true,
						Event::utility(..) => true,
						_ => false,
					};

					let filtered_accouts = if maybe {
						accounts.clone()
					} else {
						accounts.iter().filter(|id| {
	   					let target = format!("{:?}", id);
	   					event_string.as_str().contains(target.as_str())
	   				}).map(|id| id.clone()).collect::<Vec<AccountId>>()
					};

   				if filtered_accouts.len() > 0 {
   					let accounts_and_event = (filtered_accouts, record.clone());
   					targets.push(accounts_and_event)
   				}
    		}

    		if targets.len() > 0 {
    			let msg: Message = (changeset.block, targets);
    			tx.unbounded_send(msg).unwrap();
    		}
			}
		}
	}
	drop(tx);
}