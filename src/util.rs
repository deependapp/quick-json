use std::{borrow::Cow, ops::Deref};

#[derive(Clone, Copy)]
pub(crate) enum ShortOrLongRef<'s, 'l, T>
		where T: ?Sized, 'l: 's {
	Short(&'s T),
	Long(&'l T)
}

impl<'s, 'l, T> ShortOrLongRef<'s, 'l, T>
		where T: ?Sized, 'l: 's {
	pub(crate) fn map<F>(self, map: F) -> Self
			where F: for<'a> FnOnce(&'a T) -> &'a T {
		match self {
			Self::Short(short) => Self::Short(map(short)),
			Self::Long(long) => Self::Long(map(long))
		}
	}

	pub(crate) fn cow(self) -> Cow<'s, T>
			where T: ToOwned {
		match self {
			Self::Short(short) => Cow::Owned(short.to_owned()),
			Self::Long(long) => Cow::Borrowed(long)
		}
	}
}

impl<'s, 'l, T> Deref for ShortOrLongRef<'s, 'l, T>
		where T: ?Sized, 'l: 's {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		match self {
			Self::Short(short) => short,
			Self::Long(long) => long
		}
	}
}
