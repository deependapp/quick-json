use super::{/*Location,*/ ValueDeserializer};
use const_format::formatcp;
use std::{
	borrow::Cow,
	fmt::{Display, Formatter, Result as FMTResult},
	io::{Stdout, Write}
};

#[derive(Debug)]
pub enum SyntaxError {
	Unexpected {
		unexpected: Option<char>,
		expected: &'static [char],
		end_expected: bool,
		location: usize
	},

	StringUnterminated,
	StringUnexpectedControlChar,
	StringUnexpectedEscape(char),

	NumberIncomplete,
	NumberExpectedDigit
}

impl SyntaxError {
	pub fn location(&self) -> usize {
		match self {
			Self::Unexpected {location, ..} => *location,
			_ => Default::default()
		}
	}

	pub(crate) fn expect(char: Option<char>, expected: &'static [char],
			end_expected: bool, location: usize) -> Result<(), Self> {
		match char {
			Some(char) => expected.contains(&char),
			None => end_expected
		}
			.then(|| ())
			.ok_or_else(|| Self::Unexpected {
				unexpected: char,
				expected,
				end_expected,
				location
			})
	}
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum KeyKind<'s> {
	Object(Cow<'s, str>),
	Array(usize)
}

impl<'s> KeyKind<'s> {
	pub fn into_static(self) -> KeyKind<'static> {
		match self {
			KeyKind::Object(string) =>
				KeyKind::Object(Cow::Owned(string.into_owned())),
			KeyKind::Array(number) =>
				KeyKind::Array(number)
		}
	}
}

macro_rules! impl_numeric {
	($name:ident, $unsigned:literal, $signed:literal, $($constant:ident),*) => {
		pub fn $name(self) -> &'static str {
			match self {
				Self::U8 => formatcp!($unsigned, $(u8::$constant),*),
				Self::U16 => formatcp!($unsigned, $(u16::$constant),*),
				Self::U32 => formatcp!($unsigned, $(u32::$constant),*),
				Self::U64 => formatcp!($unsigned, $(u64::$constant),*),
				Self::U128 => formatcp!($unsigned, $(u128::$constant),*),
				Self::USize => formatcp!($unsigned, $(usize::$constant),*),
				Self::I8 => formatcp!($signed, $(i8::$constant),*),
				Self::I16 => formatcp!($signed, $(i16::$constant),*),
				Self::I32 => formatcp!($signed, $(i32::$constant),*),
				Self::I64 => formatcp!($signed, $(i64::$constant),*),
				Self::I128 => formatcp!($signed, $(i128::$constant),*),
				Self::ISize => formatcp!($signed, $(isize::$constant),*)
			}
		}
	}
}

macro_rules! impl_numeric_primitives {
	($($type:ty: $variant:ident),*) => {
		$(
			impl AssociatedNumeric for $type {
				const NUMERIC_PRIMITIVE: NumericPrimitive = NumericPrimitive::$variant;
			}
		)*
	}
}

pub trait AssociatedNumeric {
	const NUMERIC_PRIMITIVE: NumericPrimitive;
}

impl_numeric_primitives! {
	u8: U8,
	u16: U16,
	u32: U32,
	u64: U64,
	u128: U128,
	usize: USize,
	i8: I8,
	i16: I16,
	i32: I32,
	i64: I64,
	i128: I128,
	isize: ISize
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NumericPrimitive {
	U8,
	U16,
	U32,
	U64,
	U128,
	USize,
	I8,
	I16,
	I32,
	I64,
	I128,
	ISize
}

impl NumericPrimitive {
	impl_numeric!(min, "{}", "{}", MIN);
	impl_numeric!(max, "{}", "{}", MAX);
	impl_numeric!(noun, "unsigned {} bit integer", "signed {} bit integer", BITS);
	impl_numeric!(mention_by_noun, "an unsigned {} bit integer", "a signed {} bit integer", BITS);
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JSONType {
	/// Any object.
	Object,
	/// Any array.
	Array,
	/// Any string.
	String,
	/// Any number.
	Number,
	/// Any boolean.
	Boolean,
	/// Null.
	Null
}

impl JSONType {
	pub fn noun(self) -> &'static str {
		match self {
			Self::Object => "object",
			Self::Array => "array",
			Self::String => "string",
			Self::Number => "number",
			Self::Boolean => "boolean",
			Self::Null => "null"
		}
	}

	pub fn mention_by_noun(self) -> &'static str {
		match self {
			Self::Object => "an object",
			Self::Array => "an array",
			Self::String => "a string",
			Self::Number => "a number",
			Self::Boolean => "a boolean",
			Self::Null => "null"
		}
	}
}

impl<'d, 's> From<&ValueDeserializer<'d, 's>> for JSONType
		where 's: 'd {
	fn from(deserializer: &ValueDeserializer<'d, 's>) -> Self {
		match deserializer {
			ValueDeserializer::Object(_) => Self::Object,
			ValueDeserializer::Array(_) => Self::Array,
			ValueDeserializer::String(_) => Self::String,
			ValueDeserializer::Number(_) => Self::Number,
			ValueDeserializer::Boolean(_) => Self::Boolean,
			ValueDeserializer::Null => Self::Null
		}
	}
}

impl Display for JSONType {
	fn fmt(&self, f: &mut Formatter) -> FMTResult {
		write!(f, "{}", self.noun())
	}
}

pub trait ErrorContext<'s>: Sized {
	fn report_unknown<M>(&mut self, message: M)
		where M: ToString;
	fn report_unexpected_type(&mut self, unexpected: JSONType,
			expected: &[JSONType]) {
		match expected.len() {
			0 => panic!("cannot pass 0 expected items to report_unexpected_type"),
			1 => self.report_unknown(format!(
				"expected {}, found {}",
				expected[0].mention_by_noun(),
				unexpected.mention_by_noun()
			)),
			2 => self.report_unknown(format!(
				"expected {} or {}, found {}",
				expected[0].mention_by_noun(),
				expected[1],
				unexpected.mention_by_noun()
			)),
			len => {
				expected.iter().enumerate()
					.fold(String::from("expected "), |mut string, (index, token)| {
						match index {
							0 =>
								string.push_str(&format!("{}", token.mention_by_noun())),
							index if len - 1 == index =>
								string.push_str(&format!("or {}", token)),
							_ =>
								string.push_str(&format!(", {}", token))
						}
						string
					});
			}
		}
	}

	fn report_string_expected_borrowed(&mut self) {
		self.report_unknown(
			"expected a string borrowed from source, found an owned string")
	}

	fn report_number_overflow(&mut self, r#type: NumericPrimitive) {
		self.report_unknown(format!("value causes an integer overflow in target ({})", r#type.mention_by_noun()))
	}
	fn report_number_underflow(&mut self, r#type: NumericPrimitive) {
		self.report_unknown(format!("value causes an integer overflow in target ({})", r#type.mention_by_noun()))
	}
	fn report_number_fractional(&mut self) {
		self.report_unknown(
			"number cannot fit in target value due to having a fractional component")
	}

	fn report_missing_fields(&mut self) {
		self.report_unknown("missing fields")
	}

	fn push_key(&mut self, _key: KeyKind<'s>) {}
	fn pop_key(&mut self) {}
}

impl<'s> ErrorContext<'s> for () {
	fn report_unknown<M>(&mut self, _: M)
		where M: ToString {}
}

impl<'s> ErrorContext<'s> for Stdout {
	fn report_unknown<M>(&mut self, message: M)
			where M: ToString {
		self.lock().write_all(message.to_string().as_bytes())
			.expect("write failiure");
	}
}
