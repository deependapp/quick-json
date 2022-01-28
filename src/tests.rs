use super::from_str;
use maplit::hashmap;
use serde::Deserialize;
use std::{borrow::Cow, collections::HashMap as STDHashMap, vec::Vec as STDVec};

macro_rules! manufacture {
	() => {};
	($name:ident<$T:ty>($input:literal, $expected:expr); $($rest:tt)*) => {
		#[test]
		fn $name() {
			const INPUT: &str = $input;
			let test: $T = from_str(INPUT).unwrap();
			let expected: $T = $expected;
			assert_eq!(test, expected);
		}

		manufacture! {$($rest)*}
	}
}

type HashMap = STDHashMap<&'static str, &'static str>;
type Vec = STDVec<&'static str>;

#[derive(Debug, Deserialize, PartialEq)]
struct Complex<'s> {
	#[serde(borrow)]
	what_if: ComplexA<'s>,
	we_put: ComplexB,
	our: ComplexC
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum ComplexA<'s> {
	ThisIsAVariant,
	ThisIsAlsoAVariant(Cow<'s, str>, u32),
	IfYouHaveMoreCreativeNamesPleaseMakeAPullRequest {
		minecraft: i64,
		beds: String,
		together: &'s str
	}
}

#[derive(Debug, Deserialize, PartialEq)]
struct ComplexB(String, u8, String);

#[derive(Debug, Deserialize, PartialEq)]
struct ComplexC;

manufacture! {
	test_deserialize_float_1<f64>("-69.0", -69.0);
	test_deserialize_float_2<f64>("-123.456e7", -123.456e7);
	test_deserialize_float_3<f64>("-420", -420.);
	test_deserialize_float_4<f64>("1337.0", 1337.);
	test_deserialize_float_5<f64>("12345", 12345.);
	test_deserialize_float_6<f64>("1e20", 1e20);

	test_deserialize_bool_1<bool>("true", true);
	test_deserialize_bool_2<bool>("false", false);

	test_deserialize_map_none<HashMap>("{}", hashmap! {});
	test_deserialize_map_single<HashMap>(
		"{\"hello\":\"there\"}",
		hashmap! {"hello" => "there"}
	);
	test_deserialize_map_plural<HashMap>(
		"{\"hello\":\"there\",\"there\":\"hello\"}",
		hashmap! {"hello" => "there", "there" => "hello"}
	);

	test_deserialize_vec_none<Vec>("[]", vec![]);
	test_deserialize_vec_single<Vec>("[\"hello\"]", vec!["hello"]);
	test_deserialize_vec_plural<Vec>(
		"[\"hello\",\"there\",\"how\"]",
		vec!["hello", "there", "how"]
	);

	test_deserialize_complex_1<Complex>(
		"{\"what_if\":null,\"we_put\":[\"this is data\",7,\"big data\"],\"our\":null}",
		Complex {
			what_if: ComplexA::ThisIsAVariant,
			we_put: ComplexB("this is data".into(), 7, "big data".into()),
			our: ComplexC
		}
	);

	test_deserialize_complex_2<Complex>(
		"{\"what_if\":[\"such parse\",578924],\"we_put\":[\"this is data\",7,\"big data\"],\"our\":null}",
		Complex {
			what_if: ComplexA::ThisIsAlsoAVariant(Cow::Borrowed("such parse"), 578924),
			we_put: ComplexB("this is data".into(), 7, "big data".into()),
			our: ComplexC
		}
	);

	test_deserialize_complex_3<Complex>(
		"{\"what_if\":[\"much\\nwow\",578924],\"we_put\":[\"this is data\",7,\"big data\"],\"our\":null}",
		Complex {
			what_if: ComplexA::ThisIsAlsoAVariant(Cow::Owned("much\nwow".into()), 578924),
			we_put: ComplexB("this is data".into(), 7, "big data".into()),
			our: ComplexC
		}
	);

	test_deserialize_complex_5<Complex>(
		"{\"what_if\":{\"beds\":\"i hate making\\n\\t\\rtest data\\b\",\"together\":\"it's finally over\",\"minecraft\":-4839855329580},\"we_put\":[\"this is data\",7,\"big data\"],\"our\":null}",
		Complex {
			what_if: ComplexA::IfYouHaveMoreCreativeNamesPleaseMakeAPullRequest {
				minecraft: -4839855329580,
				beds: "i hate making\n\t\rtest data\u{8}".into(),
				together: "it's finally over"
			},
			we_put: ComplexB("this is data".into(), 7, "big data".into()),
			our: ComplexC
		}
	);
}
