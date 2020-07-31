
use futures::channel::mpsc::{ UnboundedSender, unbounded };
use futures::executor::ThreadPool;
use futures::future;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::{thread, time};
use std::collections::BTreeMap;

use codec::{ Decode, Compact };
use indicatif::{ ProgressBar, ProgressStyle };

use crate::primitives::{AccountId, EventRecord, SCAN_STEP, Hash };
use runtime::{ Event, SignedBlock };
use frame_system::{ Phase, RawEvent };
use sp_core::crypto::{ Ss58Codec };

use crate::rpc::*;
use crate::error::{ Result, Error };
use crate::store::*;
use frame_support::traits::{GetCallMetadata};
// use frame_support::dispatch::{Callable, CallableCallFor};

#[derive(Clone)]
pub struct Scanner {
  url: String,
  accounts: Vec<AccountId>,
  cursor: Arc<AtomicU64>,
  step: u64,
  tip_number: u64,
  tx: UnboundedSender<()>,
}

impl Scanner {
  pub fn new(url: String, accounts: Vec<AccountId>, tx: UnboundedSender<()>) -> Self {
    Self {
      url,
      accounts,
      cursor: Arc::new(AtomicU64::new(0)),
      step: SCAN_STEP,
      tip_number: 0,
      tx: tx,
    }
  }

  fn touch(&self, n: u32) {
    for account in self.accounts.iter() {
      let addr = account.to_ss58check();
      let store = FileStore::get(addr.as_str());
      store.update(n);
    }
  }

  async fn scan(self) {
    let mut finished = true;
    let mut pos: u32 = 0;
    let rpc = Rpc::new(self.url.clone()).await;
    loop {
      let now = self.cursor.load(Ordering::SeqCst);
      if now > self.tip_number {
        break;
      }

      let (start, end) = if finished {
        let p = self.cursor.fetch_add(self.step, Ordering::SeqCst);
        let start: u32 = p as u32;
        let end: u32 = p as u32 + self.step as u32 - 1;
        (start, end)
      } else {
        (pos, pos + self.step as u32 - 1)
      };

      let hashes = future::join(rpc.block_hash(Some(start)), rpc.block_hash(Some(end))).await;
      let (start_hash, end_hash) = match hashes {
        (Ok(Some(start)), Ok(end)) => ( start, end ),
        _ => {
          let rpc = Rpc::new(rpc.url.clone()).await;
          finished = false;
          pos = start;
          continue
        },
      };

      let mut key = sp_core::twox_128(b"System").to_vec();
      key.extend(sp_core::twox_128(b"Events").to_vec());
      let keys = vec![sp_core::storage::StorageKey(key)];
      let storage = match rpc.query_storage(keys, start_hash, end_hash).await {
        Ok(storage) => storage,
        Err(err) => match err {
          Error::Rpc(..) | Error::WsHandshake(..) => {
            let rpc = Rpc::new(rpc.url.clone()).await;
            finished = false;
            pos = start;
            continue;
          },
          _ => continue,
        },
      };

      for changeset in storage {
        let (_k, data) = changeset.changes[0].clone();
        if let Some(v) = data {
          let mut input = v.0.as_slice();
          let compact_len = <Compact<u32>>::decode(&mut input).unwrap();
          let len = compact_len.0 as usize;

          let mut records_with_idx: BTreeMap<usize, Vec<EventRecord>> = BTreeMap::new();
          for _i in 0..len {
            let record = match EventRecord::decode(&mut input) {
              Ok(v) => v,
              Err(_err) => {
                // TODO process decode error
                continue
              },
            };

            let index: usize = match record.phase {
              Phase::ApplyExtrinsic(i) => i as usize,
              _ => continue,
            };

            let maybe: bool = match record.event {
              Event::system(RawEvent::ExtrinsicFailed(..)) => true,
              Event::utility(..) => true,
              _ => false,
            };

            let event_string = format!("{:?}", record.event);
            let filtered_accouts = self.accounts.iter().filter(|id| {
              let target = format!("{:?}", id);
              event_string.as_str().contains(target.as_str())
            }).map(|id| id.clone()).collect::<Vec<AccountId>>();

            if maybe || filtered_accouts.len() > 0 {
              if records_with_idx.get(&index).is_some() {
                if let Some(v) = records_with_idx.get_mut(&index) {
                  v.push(record.clone());
                }
              } else {
                records_with_idx.insert(index, vec![record.clone()]);
              }
            }
          }

          if records_with_idx.len() > 0 {
            let block_hash = changeset.block;
            let block = match rpc.block(Some(block_hash)).await {
              Ok(signed) => signed.unwrap(),
              Err(_err) => {
                continue
              }
            };
            Self::process(block_hash, block, records_with_idx, self.accounts.clone());
          }
        }
      }

      self.touch(end);
      finished = true;
    }
    drop(self.tx);
  }

  fn process(
    block_hash: Hash, 
    block: SignedBlock, 
    event_records: BTreeMap<usize, Vec<EventRecord>>,
    accounts: Vec<AccountId>,
  ) 
  {
    let block = block.block;
    let number = block.header.number;
    let xts = block.extrinsics;

    for (index, records) in event_records.iter() {
      let is_failed = records.iter().find(|record| {
        match record.event {
          Event::system(RawEvent::ExtrinsicFailed(..)) => true,
          _ => false
        }
      }).is_some();

      let status = if is_failed {
        "failed".to_string()
      } else {
        "success".to_string()
      };


      for record in records.iter() {
        let xt = &xts[*index];
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
              index: *index as u32,
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
  }
}


pub async fn run(url: String, accounts: Vec<AccountId>) -> Result<()> {
  let threads_size = num_cpus::get() / 2;
  let cursors: Vec<u32> = accounts.iter().map(|id| FileStore::get(id.to_ss58check().as_str()).read().scanned_at).collect();
  let start_number = cursors.iter().min().unwrap_or(&0u32).clone();
  let rpc = Arc::new(Rpc::new(url.clone()).await);
  let tip_header = rpc.header(None).await?.unwrap();
  let tip_number = tip_header.number as u64;
  let (tx, mut rx) = unbounded();
  let mut scanner = Scanner::new(url.clone(), accounts, tx);
  scanner.cursor = Arc::new(AtomicU64::new(start_number as u64));
  scanner.tip_number = tip_number;

  let pool = ThreadPool::new()?;

  for _i in 0..threads_size {
    let s = scanner.clone();
    pool.spawn_ok(s.scan());
  }
  drop(scanner.tx);
  let bar = ProgressBar::new(tip_number);
  bar.set_style(ProgressStyle::default_bar()
                  .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7}")
                  .progress_chars("#>-"));
  loop {
    thread::sleep(time::Duration::from_millis(1000));
    let pos = scanner.cursor.load(Ordering::SeqCst);
    if pos > tip_number {
      bar.set_position(tip_number);
    } else {
      bar.set_position(pos);
    }
    if let Ok(None) = rx.try_next() {
      bar.finish();
      break;
    }
  }
  Ok(())
}

