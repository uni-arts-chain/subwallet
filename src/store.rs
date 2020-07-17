use serde::{Serialize, Deserialize};
use rustbreak::{FileDatabase};
use rustbreak::deser::Bincode;

// use std::path::PathBuf;
// use std::fs;

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Extrinsic {
	pub block_number: u32,
	pub block_hash: String,
	pub index: u32,
	pub signer: Option<String>,
	pub status: String,
	pub module: String,
	pub call: String,
}

impl Extrinsic {
	pub fn print(&self) {
		let i = format!("{}-{}", self.block_number, self.index);
		let flag = match self.status.as_str() {
			"success" => "✅",
			_ => "❌",
		};
		let module_call = format!("{}::{}", self.module, self.call);
		println!("{:<10} {:<55} {:<30} {:<6}", i, self.signer.as_ref().unwrap_or(&"-".to_string()), module_call, flag);
	}
}

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize, Clone, Default)]
pub struct Extrinsics {
	addr: String,
	xts: Vec<Extrinsic>,
	pub scanned_at: u32,
}

impl Extrinsics {
	pub fn new(addr: String) -> Self {
		let mut extrinsics = Self::default();
		extrinsics.addr = addr;
		extrinsics
	}

	pub fn add(&mut self, xt: Extrinsic) {
		if self.get(xt.block_number, xt.index).is_none() {
			self.scanned_at = xt.block_number;
			self.xts.push(xt);
		}
	}

	pub fn get(&self, block_number: u32, index: u32) -> Option<&Extrinsic> {
		self.xts.iter().find(|xt| xt.block_number == block_number && xt.index == index)
	}
}


pub struct FileStore(FileDatabase<Extrinsics, Bincode>);

impl FileStore {

	pub fn get(addr: &str) -> Self {
		let mut file = dirs::home_dir().unwrap();
		file.push(".subwallet");
		file.push(format!("xt-{}", addr).as_str());

		let backend = Extrinsics::new(addr.to_owned());
		let db = FileDatabase::<Extrinsics, Bincode>::from_path(file, backend).expect("Failed to initialize file database.");
		Self(db)
	}

	// pub fn init(path: Option<&str>, addr: &str) -> Self {
	// 	let file = path.map(|v| {
	// 		let mut file = PathBuf::from(v);
	// 		file.push("xt");
	// 		file
	// 	}).unwrap_or_else(|| {
	// 		let mut file = dirs::home_dir().unwrap();
	// 		file.push(".subwallet");
	// 		file.push(addr.clone());
	// 		file
	// 	});

	// 	if !file.exists() {
	// 		fs::create_dir_all(file.parent().unwrap()).expect("Failed to create store file");
	// 	}

	// 	let backend = Extrinsics::new(addr.to_owned());
	// 	let db = FileDatabase::<Extrinsics, Bincode>::from_path(file, backend).expect("Failed to initialize file database.");
	// 	Self(db)
	// }

	pub fn load(&self) {
		let _ = self.0.load();
	}

	pub fn save(&self, tx: Extrinsic) {
		self.load();
		self.0.write(|backend| {
			backend.add(tx)
		}).expect("Failed to write Extrinsic");
		self.0.save().expect("Failed to save");
	}

	pub fn update(&self, scanned_at: u32) {
		self.load();
		self.0.write(|backend| {
			backend.scanned_at = scanned_at
		}).expect("Failed to update Extrinsics");
		self.0.save().expect("Failed to save");
	}

	// pub fn read(&self, tx_hash: &str) -> Option<Extrinsic> {
	// 	self.load();
	// 	let backend = self.0.borrow_data().expect("Failed to read data");
	// 	let v = backend.get(tx_hash);
	// 	match v {
	// 		Some(tx) => Some(tx.clone()),
	// 		None => None
	// 	}
	// }


	pub fn read(&self) -> Extrinsics {
		self.load();
		self.0.borrow_data().expect("Failed to read data").clone()
	}

	pub fn read_all(&self) -> Vec<Extrinsic> {
		self.load();
		let backend = self.0.borrow_data().expect("Failed to read data");
		backend.xts.clone()
	}
}