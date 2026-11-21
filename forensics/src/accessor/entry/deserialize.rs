use serde::{
    Deserializer,
    de::{self, Visitor},
};
// https://dev.to/sgchris/using-phantomdata-and-zero-sized-types-1i2e
use std::marker::PhantomData;

/// Struct that implements deserialize JSON number (or string)
struct UintVisitor<T>(PhantomData<T>);

impl<T: TryFrom<u64>> UintVisitor<T> {
    /// Convert JSON number (or string) to `T`
    fn convert<E: de::Error>(value: u64) -> Result<T, E> {
        T::try_from(value).map_err(|_err| E::custom(format!("integer {value} is out of range")))
    }
}

impl<'a, T: TryFrom<u64>> Visitor<'a> for UintVisitor<T> {
    type Value = T;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("an unsigned integer, a float, or a numeric string")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Self::convert(v)
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let value = u64::try_from(v)
            .map_err(|err| E::custom(format!("integer {v} must not be negative: {err:?}")))?;
        Self::convert(value)
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let max_number = 9007199254740991.0;
        if !(0.0..=max_number).contains(&v) || v.fract() != 0.0 {
            return Err(E::custom(format!("float {v} is not an unsigned integer")));
        }

        Self::convert(v as u64)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let value = v
            .trim()
            .parse::<u64>()
            .map_err(|err| E::custom(format!("invalid integer string {v}: {err:?}")))?;

        Self::convert(value)
    }
}

/// Deserialize unsigned integer from JSON (float, integer, or string)
///
/// Used via `#[serde(deserialize_with = "deserialize_uint")]`
pub(crate) fn deserialize_uint<'de, D, T>(deserialize: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: TryFrom<u64>,
{
    deserialize.deserialize_any(UintVisitor(PhantomData))
}

#[cfg(test)]
mod tests {
    use super::deserialize_uint;
    use serde::Deserialize;
    use serde_json::{Error, Value, json};

    #[derive(Debug, Deserialize, PartialEq)]
    struct SampleTest {
        #[serde(deserialize_with = "deserialize_uint")]
        small: u16,
        #[serde(deserialize_with = "deserialize_uint")]
        medium: u32,
        #[serde(deserialize_with = "deserialize_uint")]
        large: u64,
    }

    fn parse(value: Value) -> Result<SampleTest, Error> {
        serde_json::from_value(value)
    }

    #[test]
    fn test_int_float_string() {
        let golden = SampleTest {
            small: 7,
            medium: 3747804916,
            large: 3722112,
        };

        let ints = parse(json!({"small": 7, "medium": 3747804916u32, "large": 3722112})).unwrap();
        assert_eq!(ints, golden);

        let floats =
            parse(json!({"small": 7.0, "medium": 3747804916.0, "large": 3722112.0})).unwrap();
        assert_eq!(floats, golden);

        let strings =
            parse(json!({"small": "7", "medium": "3747804916", "large": "3722112"})).unwrap();
        assert_eq!(strings, golden);

        let results =
            parse(json!({"small": 1, "medium": 3747804916u32, "large": 18446744073709551615u64}))
                .unwrap();
        assert_eq!(results.large, 18446744073709551615);
    }

    #[test]
    fn test_bad_values() {
        let mut test = json!({"small": 1.5, "medium": 1, "large": 1});
        let mut err = parse(test).unwrap_err();
        assert!(format!("{err:?}").contains("float 1.5 is not an unsigned integer"));

        test = json!({"small": -1, "medium": 1, "large": 1});
        err = parse(test).unwrap_err();
        assert!(format!("{err:?}").contains("integer -1 must not be negative"));

        test = json!({"small": 70000, "medium": 1, "large": 1});
        err = parse(test).unwrap_err();
        assert!(format!("{err:?}").contains("integer 70000 is out of range"));

        test = json!({"small": 1, "medium": 5000000000u64, "large": 1});
        err = parse(test).unwrap_err();
        assert!(format!("{err:?}").contains("integer 5000000000 is out of range"));

        test = json!({"small": 1, "medium": 1, "large": 1.8446744073709552e19});
        err = parse(test).unwrap_err();
        assert!(
            format!("{err:?}").contains("float 18446744073709552000 is not an unsigned integer")
        );

        test = json!({"small": 1, "medium": 1, "large": "abc"});
        err = parse(test).unwrap_err();
        assert!(format!("{err:?}").contains("invalid integer string abc"));
    }
}
