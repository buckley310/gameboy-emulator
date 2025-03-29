use serde::Deserialize;
use std::collections::HashMap;

const OPCODES: &str = include_str!("../opcodes.json");

#[derive(Deserialize)]
pub struct Operand {
	pub name: String,
	pub bytes: Option<u8>,
	pub immediate: bool,
}

#[derive(Deserialize)]
pub struct Opcode {
	pub bytes: u16,
	pub cycles: Vec<u64>,
	pub flags: HashMap<String, String>,
	pub immediate: bool,
	pub mnemonic: String,
	pub operands: Vec<Operand>,
}

#[derive(Deserialize)]
pub struct Opcodes {
	pub unprefixed: HashMap<String, Opcode>,
	pub cbprefixed: HashMap<String, Opcode>,
}
impl Opcodes {
	pub fn new() -> Opcodes {
		serde_json::from_str(OPCODES).unwrap()
	}
}
