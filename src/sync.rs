
use futures::channel::mpsc::{ UnboundedSender, unbounded };
use futures::executor::ThreadPool;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::{thread, time};

use codec::{ Decode, Compact };
use indicatif::{ ProgressBar, ProgressStyle };

use crate::primitives::{AccountId, EventRecord, SCAN_STEP, AccountsAndEvent, Hash };
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

      let start_hash = match rpc.block_hash(Some(start)).await {
        Ok(hash) => match hash {
          Some(h) => h,
          None => break,
        },
        Err(_) => {
          let rpc = Rpc::new(rpc.url.clone()).await;
          finished = false;
          pos = start;
          continue;
        },
      };

      let end_hash = match rpc.block_hash(Some(end)).await {
        Ok(hash) => hash,
        Err(_) => {
          let rpc = Rpc::new(rpc.url.clone()).await;
          finished = false;
          pos = start;
          continue;
        }
      };

      let mut key = sp_core::twox_128(b"System").to_vec();
      key.extend(sp_core::twox_128(b"Events").to_vec());
      let keys = vec![sp_core::storage::StorageKey(key)];
      let storage = match rpc.query_storage(keys, start_hash, end_hash).await {
        Ok(storage) => storage,
        Err(err) => match err {
          Error::Rpc(..) | Error::WsHandshake(..) | Error::WsError(..) => {
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

          let mut targets = Vec::new();
          for _i in 0..len {
            let record = match EventRecord::decode(&mut input) {
              Ok(v) => v,
              Err(_err) => {
                // TODO process decode error
                continue
              },
            };
            let event_string = format!("{:?}", record.clone().event);

            let maybe: bool = match record.event {
              Event::system(RawEvent::ExtrinsicFailed(..)) => true,
              Event::utility(..) => true,
              _ => false,
            };

            let filtered_accouts = if maybe {
              self.accounts.clone()
            } else {
              self.accounts.iter().filter(|id| {
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
            let block_hash = changeset.block;
            let block = match rpc.block(Some(block_hash)).await {
              Ok(signed) => signed.unwrap(),
              Err(_err) => {
                continue
              }
            };
            Self::process(block_hash, block, targets);
          }
        }
      }

      self.touch(end);
      finished = true;
    }
    drop(self.tx);
  }

  fn process(block_hash: Hash, block: SignedBlock, targets: Vec<AccountsAndEvent>) {
    let block = block.block;
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

  drop(rpc);

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