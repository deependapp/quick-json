use super::{
	error::{AssociatedNumeric, ErrorContext, JSONType, KeyKind, SyntaxError},
	Number, ValueDeserializer
};
use std::{borrow::Cow, collections::HashMap, hash::Hash};

pub use qj_derive::Deserialize;

pub trait Deserialize<'s, E>: 's
		where E: ErrorContext<'s> {
	fn deserialize<'d>(value: ValueDeserializer<'d, 's>, error_context: &mut E)
		-> Result<Option<Self>, SyntaxError> where Self: Sized;
}

macro_rules! deserializer_match {
	(in $value:expr, $error:expr; $($type:ident($bind:pat) => $action:expr),*) => {
		match $value {
			$(ValueDeserializer::$type($bind) => $action,)*
			unexpected => {
				$error
					.report_unexpected_type(unexpected.kind(), &[$(JSONType::$type),*]);
				Ok(None)
			}
		}
	}
}

macro_rules! number_to_int {
	($number:expr, $error_context:expr, $int:ty) => {
		{
			let float = number_to_float($number.accept()?);
			let error_context = $error_context;
			if float.round() != float {
				error_context.report_number_fractional();
				Ok(None)
			} else {
				if float > (<$int>::MAX as f64) {
					error_context.report_number_overflow(<$int>::NUMERIC_PRIMITIVE);
					Ok(None)
				} else if float < (<$int>::MIN as f64) {
					error_context.report_number_underflow(<$int>::NUMERIC_PRIMITIVE);
					Ok(None)
				} else {
					Ok(Some(float as $int))
				}
			}
		}
	}
}

macro_rules! impl_integers {
	($($integer:ty),*) => {
		$(
			impl<'s, E> Deserialize<'s, E> for $integer
					where E: ErrorContext<'s> {
				#[inline]
				fn deserialize<'d>(value: ValueDeserializer<'d, 's>,
						error_context: &mut E) -> Result<Option<Self>, SyntaxError> {
					deserializer_match! {in value, error_context;
						Number(number) => number_to_int!(number, error_context, $integer)
					}
				}
			}
		)*
	}
}

#[inline]
fn number_to_float(number: Number) -> f64 {
	let mut result = number.source[number.base].parse()
		.expect("number base was parsed incorrectly");
	let fraction = number.fraction
		.map(|fraction| {
			number.source[fraction.start - 1..fraction.end].parse()
				.expect("number fraction was parsed incorrectly")
		})
		.unwrap_or(0.);
	let mut exponent = number.exponent
		.map(|exponent| {
			number.source[exponent].parse()
				.expect("number fraction was parsed incorrectly")
		})
		.unwrap_or(0.);

	if !number.base_positive {result *= -1.}
	if !number.exponent_positive {exponent *= -1.}

	result += fraction;
	result *= f64::powf(10., exponent);
	result
}

impl<'s, E> Deserialize<'s, E> for String
		where E: ErrorContext<'s> {
	#[inline]
	fn deserialize<'d>(value: ValueDeserializer<'d, 's>, error_context: &mut E)
			-> Result<Option<Self>, SyntaxError> {
		Cow::deserialize(value, error_context)
			.map(|value| value.map(Cow::into_owned))
	}
}

impl<'s, E> Deserialize<'s, E> for Cow<'s, str>
		where E: ErrorContext<'s> {
	#[inline]
	fn deserialize<'d>(value: ValueDeserializer<'d, 's>, error_context: &mut E)
			-> Result<Option<Self>, SyntaxError> {
		deserializer_match! {in value, error_context;
			String(string) => Ok(Some(string.accept()?))
		}
	}
}

impl<'s, E> Deserialize<'s, E> for &'s str
		where E: ErrorContext<'s> {
	#[inline]
	fn deserialize<'d>(value: ValueDeserializer<'d, 's>, error_context: &mut E)
			-> Result<Option<Self>, SyntaxError> {
		deserializer_match! {in value, error_context;
			String(string) => match string.accept()? {
				Cow::Borrowed(string) => Ok(Some(string)),
				Cow::Owned(_) => {
					error_context.report_string_expected_borrowed();
					Ok(None)
				}
			}
		}
	}
}

impl_integers! {
	usize, u8, u16, u32, u64, u128,
	isize, i8, i16, i32, i64, i128
}

impl<'s, E> Deserialize<'s, E> for f32
		where E: ErrorContext<'s> {
	#[inline]
	fn deserialize<'d>(value: ValueDeserializer<'d, 's>, error_context: &mut E)
			-> Result<Option<Self>, SyntaxError> {
		deserializer_match! {in value, error_context;
			Number(number) =>
				Ok(Some(number_to_float(number.accept()?) as f32))
		}
	}
}

impl<'s, E> Deserialize<'s, E> for f64
		where E: ErrorContext<'s> {
	#[inline]
	fn deserialize<'d>(value: ValueDeserializer<'d, 's>, error_context: &mut E)
			-> Result<Option<Self>, SyntaxError> {
		deserializer_match! {in value, error_context;
			Number(number) =>
				Ok(Some(number_to_float(number.accept()?)))
		}
	}
}

impl<'s, T, E> Deserialize<'s, E> for Option<T>
		where T: Deserialize<'s, E>, E: ErrorContext<'s> {
	#[inline]
	fn deserialize<'d>(value: ValueDeserializer<'d, 's>, error_context: &mut E)
			-> Result<Option<Self>, SyntaxError> {
		match value {
			ValueDeserializer::Null => Ok(Some(None)),
			value => Ok(T::deserialize(value, error_context)?.map(Some))
		}
	}
}

impl<'s, T, E> Deserialize<'s, E> for Vec<T>
		where T: Deserialize<'s, E>, E: ErrorContext<'s> {
	#[inline]
	fn deserialize<'d>(value: ValueDeserializer<'d, 's>, error_context: &mut E)
			-> Result<Option<Self>, SyntaxError> {
		deserializer_match! {in value, error_context;
			Array(mut array) => {
				let mut result = Vec::new();
				let mut index = 0;
				while let Some(value) = array.next_entry()? {
					error_context.push_key(KeyKind::Array(index));
					if let Some(value) = T::deserialize(value, error_context)? {
						result.push(value);
					}
					error_context.pop_key();
					index += 1;
				}
				Ok(Some(result))
			}
		}
	}
}

impl<'s, T, U, E> Deserialize<'s, E> for HashMap<T, U>
		where T: From<Cow<'s, str>> + Hash + Eq + 's, U: Deserialize<'s, E>,
			E: ErrorContext<'s> {
	#[inline]
	fn deserialize<'d>(value: ValueDeserializer<'d, 's>, error_context: &mut E)
			-> Result<Option<Self>, SyntaxError> {
		deserializer_match! {in value, error_context;
			Object(mut object) => {
				let mut result = HashMap::new();
				while let Some(entry) = object.next_entry()? {
					let (name, value) = entry.accept()?;
					error_context.push_key(KeyKind::Object(name.clone()));
					if let Some(value) = U::deserialize(value, error_context)? {
						result.insert(name.into(), value);
					}
					error_context.pop_key();
				}
				Ok(Some(result))
			}
		}
	}
}
