use std::path::Path;
use std::{collections::HashMap, env::current_dir};
use std::fs::read_to_string;

use json::deserialize::{error::{ErrorContext, KeyKind}, from_str_default};
use json::deserialize::r#trait::Deserialize;

#[derive(Debug)]
struct Error<'s> {
	message: String,
	path: Vec<KeyKind<'s>>
}

#[derive(Default, Debug)]
struct Errors<'s> {
	path: Vec<KeyKind<'s>>,
	errors: Vec<Error<'s>>
}

impl<'s> Errors<'s> {
	fn finalize(self) -> Vec<Error<'s>> {
		self.errors
	}
}

impl<'s> ErrorContext<'s> for Errors<'s> {
	fn report_unknown<M>(&mut self, message: M)
			where M: ToString {
		self.errors.push(Error {
			message: message.to_string(),
			path: self.path.clone()
		})
	}

	fn push_key(&mut self, key: KeyKind<'s>) {
		self.path.push(key);
	}

	fn pop_key(&mut self) {
		self.path.pop();
	}
}

#[derive(Debug, Deserialize)]
struct MyThing {
	a: f64,
	x: String,
	y: HashMap<String, String>,
	z: Option<String>
}

fn main() {
	let input = Path::new(file!()).parent().unwrap().join("play.json");
	let input = read_to_string(input).unwrap();
	let (result, errors) = from_str_default::<MyThing, Errors>(&input);
	println!("{:?}", result);
	println!("{:#?}", errors.finalize());
}
