pub mod error;
pub mod r#trait;

use super::util::ShortOrLongRef;
use self::{error::{ErrorContext, SyntaxError, JSONType}, r#trait::Deserialize};
use std::{borrow::Cow, mem::{replace, forget}, ops::{Deref, DerefMut, Range}};

pub fn from_str_default<'s, T, E>(str: &'s str)
		-> (Result<Option<T>, SyntaxError>, E)
			where T: Deserialize<'s, E>, E: ErrorContext<'s> + Default + 's {
	let mut deserializer = Deserializer::new(str);
	let mut error_context = E::default();
	let result = ValueDeserializer::new(&mut deserializer)
		.and_then(|deserializer| T::deserialize(deserializer, &mut error_context));
	(result, error_context)
}

#[inline]
fn copy_range<T>(range: &Range<T>) -> Range<T>
		where T: Copy {
	let Range {start, end} = range;
	*start..*end
}

// TODO: Even more cheese
/*
#[derive(Clone, Copy, Debug, Default)]
pub struct Location {
	pub index: usize,
	pub line: usize,
	pub column: usize
}

impl Location {
	pub fn increment_line_mut(&mut self) {
		self.column = 0;
		self.line += 1;
	}
}*/

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Number<'s> {
	pub source: Cow<'s, str>,
	pub base: Range<usize>,
	pub base_positive: bool,
	pub fraction: Option<Range<usize>>,
	pub exponent: Option<Range<usize>>,
	pub exponent_positive: bool
}

impl<'s> Number<'s> {
	#[inline]
	pub fn base(&self) -> &str {
		&self.source[copy_range(&self.base)]
	}

	#[inline]
	pub fn fraction(&self) -> Option<&str> {
		self.fraction.as_ref().map(|fraction| &self.source[copy_range(fraction)])
	}

	#[inline]
	pub fn exponent(&self) -> Option<&str> {
		self.exponent.as_ref().map(|exponent| &self.source[copy_range(exponent)])
	}
}

#[derive(Debug)]
pub struct Deserializer<'s> {
	input: &'s str,
	index: usize,
	consumed: usize
	//Location,
	//consumed: Location
	// TODO: Cheese
}

impl<'s> Deserializer<'s> {
	pub fn new(input: &'s str) -> Self {
		Self {input, index: 0, consumed: 0}
	}

	fn next_char(&mut self) -> Option<char> {
		let char = self.input[self.index..].chars().next()?;
		self.index += char.len_utf8();
		//if char == '\n' {self.index.increment_line_mut()}
		Some(char)
	}

	fn next_non_whitespace_char(&mut self) -> Option<char> {
		match self.next_char() {
			Some(char) if char.is_whitespace() =>
				self.next_non_whitespace_char(),
			Some(char) => Some(char),
			None => None
		}
	}

	fn next_str<'a>(&mut self, bytes: usize) -> ShortOrLongRef<'a, 's, str> {
		let str = &self.input[self.index..bytes + self.index];
		self.index += str.len();
		//self.index.column += str.len();
		//str.chars().filter(|char| *char == '\n')
			//.for_each(|_| self.index.increment_line_mut());
		ShortOrLongRef::Long(str)
	}

	fn commit(&mut self) -> &mut Self {
		self.input = &self.input[self.index..];
		self.consumed += self.index;
		self.reset();
		self
	}

	fn reset(&mut self) -> &mut Self {
		self.index = 0;
		self
	}

	fn back(&mut self, bytes: usize) {
		// TODO: Doesn't account for lines in location.
		self.index -= bytes;
		//self.index.column -= bytes;
	}

	fn buffer<'a>(&mut self) -> ShortOrLongRef<'a, 's, str> {
		ShortOrLongRef::Long(&self.input[..self.index])
	}

	fn consumed(&self) -> usize {
		self.index + self.consumed
	}
}

#[derive(Debug)]
pub enum ValueDeserializer<'d, 's>
		where 's: 'd {
	Object(ObjectDeserializer<'d, 's>),
	Array(ArrayDeserializer<'d, 's>),
	String(StringDeserializer<'d, 's>),
	Number(NumberDeserializer<'d, 's>),
	Boolean(bool),
	Null
}

impl<'d, 's> ValueDeserializer<'d, 's>
		where 's: 'd {
	pub fn new(deserializer: &'d mut Deserializer<'s>)
			-> Result<Self, SyntaxError> {
		match deserializer.next_non_whitespace_char() {
			Some('{') =>
				Ok(Self::Object(ObjectDeserializer::new(deserializer.reset()))),
			Some('[') =>
				Ok(Self::Array(ArrayDeserializer::new(deserializer.reset()))),
			Some('"') =>
				Ok(Self::String(StringDeserializer::new(deserializer.reset()))),
			Some('0'..='9' | '-') =>
				Ok(Self::Number(NumberDeserializer::new(deserializer.reset()))),
			Some('f') if &*deserializer.next_str(4) == "alse" =>
				{deserializer.commit(); Ok(Self::Boolean(false))},
			Some('t') if &*deserializer.next_str(3) == "rue" =>
				{deserializer.commit(); Ok(Self::Boolean(true))},
			Some('n') if &*deserializer.next_str(3) == "ull" =>
				{deserializer.commit(); Ok(Self::Null)},

			unexpected => {
				deserializer.reset();
				Err(SyntaxError::Unexpected {
					unexpected,
					expected: &['{', '[', '"', '0', '1', '2', '3', '4', '5',
						'6', '7', '8', '9', 'f', 't', 'n'],
					end_expected: false,
					location: deserializer.consumed()
				})
			}
		}
	}

	pub fn kind(&self) -> JSONType {
		self.into()
	}
}

#[derive(Debug)]
pub struct ObjectDeserializer<'d, 's>
		where 's: 'd {
	deserializer: &'d mut Deserializer<'s>,
	past_first: bool
}

impl<'d, 's> ObjectDeserializer<'d, 's>
		where 's: 'd {
	#[inline]
	fn new(deserializer: &'d mut Deserializer<'s>) -> Self {
		Self {deserializer, past_first: false}
	}

	/*
	fn into_source(self) -> Cow<'s, str> {}
	*/

	/*
	#[inline]
	fn skip_internal(&mut self) {
		if replace(&mut self.past_first, true) {
			match self.deserializer.next_char() {
				Some('}') =>
					self.deserializer.back(1),
				Some(',') => ObjectFieldDeserializer::new(&mut *self.deserializer).skip_internal(),

				unexpected => {
					self.deserializer.reset();
					Err(SyntaxError::Unexpected {
						unexpected,
						expected: &['}', ','],
						end_expected: false
					})
				}
			}
		}
	}*/

	#[inline(always)]
	pub fn next_entry<'n>(&'n mut self)
			-> Result<Option<ObjectFieldDeserializer<'n, 's>>, SyntaxError> {
		if replace(&mut self.past_first, true) { // ,"data":... or }
			match self.deserializer.next_non_whitespace_char() {
				Some('}') =>
					{self.deserializer.reset(); Ok(None)},
				Some(',') =>
					Ok(Some(ObjectFieldDeserializer::new(self.deserializer.commit()))),

				unexpected => {
					self.deserializer.reset();
					Err(SyntaxError::Unexpected {
						unexpected,
						expected: &['}', ','],
						end_expected: false,
						location: self.consumed()
					})
				}
			}
		} else { // {"data"...
			SyntaxError::expect(self.deserializer.next_non_whitespace_char(),
				&['{'], false, self.consumed())?;
			self.deserializer.commit();

			let char = self.deserializer.next_non_whitespace_char();
			self.deserializer.reset();
			if let Some('}') = char {
				Ok(None)
			} else {
				Ok(Some(ObjectFieldDeserializer::new(self.deserializer)))
			}
		}
	}
}

impl<'d, 's> Drop for ObjectDeserializer<'d, 's>
		where 's: 'd {
	#[inline]
	fn drop(&mut self) {
		while let Ok(Some(_)) = self.next_entry() {}
		let _ = self.deserializer.next_non_whitespace_char(); // Token::RightCurly
		self.deserializer.commit();
	}
}

impl<'d, 's> Deref for ObjectDeserializer<'d, 's>
		where 's: 'd {
	type Target = Deserializer<'s>;

	#[inline]
	fn deref(&self) -> &Self::Target {
		&self.deserializer
	}
}

impl<'d, 's> DerefMut for ObjectDeserializer<'d, 's>
		where 's: 'd {
	#[inline]
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.deserializer
	}
}

#[derive(Debug)]
pub struct ObjectFieldDeserializer<'d, 's>(&'d mut Deserializer<'s>)
	where 's: 'd;

impl<'d, 's> ObjectFieldDeserializer<'d, 's>
		where 's: 'd {
	#[inline]
	fn new(deserializer: &'d mut Deserializer<'s>) -> Self {
		Self(deserializer)
	}

	#[inline]
	pub fn accept(self)
			-> Result<(Cow<'s, str>, ValueDeserializer<'d, 's>), SyntaxError> {
		let deserializer = self.0 as *mut _;
		forget(self);
		unsafe {Self::accept_internal(&mut *deserializer)}
	}

	#[inline(always)]
	fn accept_internal(deserializer: &'d mut Deserializer<'s>)
			-> Result<(Cow<'s, str>, ValueDeserializer<'d, 's>), SyntaxError> {
		let name = StringDeserializer::new(deserializer).accept()?;
		SyntaxError::expect(deserializer.next_non_whitespace_char(),
			&[':'], false, deserializer.consumed())?;
		deserializer.commit();
		Ok((name, ValueDeserializer::new(deserializer)?))
	}
}

impl<'d, 's> Drop for ObjectFieldDeserializer<'d, 's>
		where 's: 'd {
	fn drop<'u>(&'u mut self) {
		let deserializer = self.0 as *mut _;
		forget(self);
		let _ = unsafe {Self::accept_internal(&mut *deserializer)};
	}
}

impl<'d, 's> Deref for ObjectFieldDeserializer<'d, 's>
		where 's: 'd {
	type Target = Deserializer<'s>;

	#[inline]
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<'d, 's> DerefMut for ObjectFieldDeserializer<'d, 's>
		where 's: 'd {
	#[inline]
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

#[derive(Debug)]
pub struct ArrayDeserializer<'d, 's>
		where 's: 'd {
	deserializer: &'d mut Deserializer<'s>,
	past_first: bool
}

impl<'d, 's> ArrayDeserializer<'d, 's>
		where 's: 'd {
	#[inline]
	fn new(deserializer: &'d mut Deserializer<'s>) -> Self {
		Self {deserializer, past_first: false}
	}

	#[inline(always)]
	pub fn next_entry<'n>(&'n mut self)
			-> Result<Option<ValueDeserializer<'n, 's>>, SyntaxError> {
		if replace(&mut self.past_first, true) { // ,"data"... or ]
			match self.deserializer.next_non_whitespace_char() {
				Some(']') =>
					{self.deserializer.reset(); Ok(None)},
				Some(',') =>
					Ok(Some(ValueDeserializer::new(self.deserializer.commit())?)),

				unexpected => {
					self.deserializer.reset();
					Err(SyntaxError::Unexpected {
						unexpected,
						expected: &[']', ','],
						end_expected: false,
						location: self.consumed()
					})
				}
			}
		} else { // ["data"...
			SyntaxError::expect(self.deserializer.next_non_whitespace_char(),
				&['['], false, self.consumed())?;
			self.deserializer.commit();

			let char = self.deserializer.next_non_whitespace_char();
			self.deserializer.reset();
			if let Some(']') = char {
				Ok(None)
			} else {
				Ok(Some(ValueDeserializer::new(self.deserializer)?))
			}
		}
	}
}

impl<'d, 's> Drop for ArrayDeserializer<'d, 's>
		where 's: 'd {
	#[inline]
	fn drop(&mut self) {
		while let Ok(Some(_)) = self.next_entry() {}
		let _ = self.deserializer.next_non_whitespace_char(); // Token::RightCurly
		self.deserializer.commit();
	}
}

impl<'d, 's> Deref for ArrayDeserializer<'d, 's>
		where 's: 'd {
	type Target = Deserializer<'s>;

	#[inline]
	fn deref(&self) -> &Self::Target {
		&self.deserializer
	}
}

impl<'d, 's> DerefMut for ArrayDeserializer<'d, 's>
		where 's: 'd {
	#[inline]
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.deserializer
	}
}

#[derive(Debug)]
pub struct StringDeserializer<'d, 's>(&'d mut Deserializer<'s>)
	where 's: 'd;

impl<'d, 's> StringDeserializer<'d, 's>
		where 's: 'd {
	#[inline]
	fn new(deserializer: &'d mut Deserializer<'s>) -> Self {
		Self(deserializer)
	}

	#[inline]
	pub fn accept(mut self) -> Result<Cow<'s, str>, SyntaxError> {
		let result = self.accept_internal();
		forget(self);
		result
	}

	#[inline(always)]
	fn accept_internal(&mut self) -> Result<Cow<'s, str>, SyntaxError> {
		SyntaxError::expect(self.0.next_non_whitespace_char(),
			&['"'], false, self.consumed())?;
		self.0.commit();

		let mut owned = None;
		let result = loop {
			match self.0.next_char() {
				Some('\\') => {
					let owned = owned.get_or_insert(String::from(&*self.0.buffer()));

					match self.0.next_char() {
						Some(char @ ('"' | '\\' | '/')) =>
							owned.push(char),

						Some('b') => owned.push('\u{8}'),
						Some('f') => owned.push('\u{C}'),
						Some('n') => owned.push('\n'),
						Some('r') => owned.push('\r'),
						Some('t') => owned.push('\t'),

						Some('u') => todo!(),

						Some(escape) =>
							break Err(SyntaxError::StringUnexpectedEscape(escape)),
						None =>
							break Err(SyntaxError::StringUnterminated)
					}
				},

				Some('"') => match owned {
					Some(owned) => break Ok(Cow::Owned(owned)),
					None => break Ok(self.0.buffer()
						.map(|buffer| &buffer[..buffer.len() - 1]).cow())
				},

				Some(char) if char.is_control() =>
					break Err(SyntaxError::StringUnexpectedControlChar),
				Some(char) => if let Some(owned) = &mut owned {owned.push(char);},
				None => break Err(SyntaxError::StringUnterminated)
			}
		};

		self.0.commit();
		result
	}
}

impl<'d, 's> Drop for StringDeserializer<'d, 's>
		where 's: 'd {
	fn drop(&mut self) {
		let _ = self.accept_internal();
	}
}

impl<'d, 's> Deref for StringDeserializer<'d, 's>
		where 's: 'd {
	type Target = Deserializer<'s>;

	#[inline]
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<'d, 's> DerefMut for StringDeserializer<'d, 's>
		where 's: 'd {
	#[inline]
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

#[derive(Debug)]
pub struct NumberDeserializer<'d, 's>(&'d mut Deserializer<'s>)
	where 's: 'd;

impl<'d, 's> NumberDeserializer<'d, 's>
		where 's: 'd {
	#[inline]
	fn new(deserializer: &'d mut Deserializer<'s>) -> Self {
		Self(deserializer)
	}

	#[inline]
	pub fn accept(mut self) -> Result<Number<'s>, SyntaxError> {
		let result = self.accept_internal();
		forget(self);
		result
	}

	#[inline(always)]
	fn accept_internal(&mut self) -> Result<Number<'s>, SyntaxError> {
		self.next_non_whitespace_char();
		self.back(1);
		self.commit();

		let base_positive = match self.next_char() {
			Some('-') => Ok(false),
			Some(_) => {self.back(1); Ok(true)},
			None => Err(SyntaxError::Unexpected {
				unexpected: None,
				expected: &['-', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
				end_expected: false,
				location: self.consumed()
			})
		}?;

		let base = self.buffer().len();
		let base = match self.next_char() {
			Some('0') => Ok(base..self.buffer().len()),
			Some('1'..='9') => loop {
				match self.next_char() {
					Some('0'..='9') => (),
					Some(_) => {self.back(1); break Ok(base..self.buffer().len())},
					None => break Ok(base..self.buffer().len())
				}
			},
			unexpected => Err(SyntaxError::Unexpected {
				unexpected,
				expected: &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
				end_expected: false,
				location: self.consumed()
			})
		}?;

		let fraction = match self.next_char() {
			Some('.') => {
				let fraction = self.buffer().len();
				match self.next_char() {
					Some('0'..='9') => loop {
						match self.next_char() {
							Some('0'..='9') => (),
							Some(_) => {
								self.back(1);
								break Ok(Some(fraction..self.buffer().len()))
							},
							None => break Ok(Some(fraction..self.buffer().len()))
						}
					},
					unexpected => Err(SyntaxError::Unexpected {
						unexpected,
						expected: &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
						end_expected: false,
						location: self.consumed()
					})
				}
			},
			Some(_) => {self.back(1); Ok(None)},
			None => Ok(None)
		}?;

		let exponent = match self.next_char() {
			Some('e' | 'E') => {
				let positive = match self.next_char() {
					Some('+') => Ok(true),
					Some('-') => Ok(false),
					Some(_) => {self.back(1); Ok(true)},
					None => Err(SyntaxError::Unexpected {
						unexpected: None,
						expected: &['+', '-',
							'0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
						end_expected: false,
						location: self.consumed()
					})
				}?;
	
				let exponent = self.buffer().len();
				match self.next_char() {
					Some('0'..='9') => loop {
						match self.next_char() {
							Some('0'..='9') => (),
							Some(_) => {
								self.back(1);
								break Ok(Some((exponent..self.buffer().len(), positive)))
							},
							None => break Ok(Some((exponent..self.buffer().len(), positive)))
						}
					},
					unexpected => Err(SyntaxError::Unexpected {
						unexpected,
						expected: &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
						end_expected: false,
						location: self.consumed()
					})
				}
			},
			Some(_) => {self.back(1); Ok(None)},
			None => Ok(None)
		}?;

		let (exponent, exponent_positive) = exponent.map_or((None, false),
			|(exponent, exponent_positive)| (Some(exponent), exponent_positive));
		let source = self.buffer().cow();
		self.commit();

		Ok(Number {source, base, base_positive,
			fraction, exponent, exponent_positive})
	}
}

impl<'d, 's> Drop for NumberDeserializer<'d, 's>
		where 's: 'd {
	fn drop(&mut self) {
		let _ = self.accept_internal();
	}
}

impl<'d, 's> Deref for NumberDeserializer<'d, 's>
		where 's: 'd {
	type Target = Deserializer<'s>;

	#[inline]
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<'d, 's> DerefMut for NumberDeserializer<'d, 's>
		where 's: 'd {
	#[inline]
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}
