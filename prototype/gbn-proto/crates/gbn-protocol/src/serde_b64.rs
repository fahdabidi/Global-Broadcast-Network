use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use serde::{de::Error as _, Deserialize, Deserializer, Serializer};

fn serialize_array<const N: usize, S>(value: &[u8; N], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&STANDARD_NO_PAD.encode(value))
}

fn deserialize_array<'de, const N: usize, D>(deserializer: D) -> Result<[u8; N], D::Error>
where
    D: Deserializer<'de>,
{
    let encoded = String::deserialize(deserializer)?;
    let decoded = STANDARD_NO_PAD
        .decode(encoded.as_bytes())
        .map_err(D::Error::custom)?;
    decoded.try_into().map_err(|decoded: Vec<u8>| {
        D::Error::custom(format!(
            "expected {} decoded bytes, got {}",
            N,
            decoded.len()
        ))
    })
}

pub mod bytes {
    use super::*;

    pub fn serialize<S>(value: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD_NO_PAD.encode(value))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        STANDARD_NO_PAD
            .decode(encoded.as_bytes())
            .map_err(D::Error::custom)
    }
}

macro_rules! fixed_array_module {
    ($name:ident, $size:expr) => {
        pub mod $name {
            use super::*;

            pub fn serialize<S>(value: &[u8; $size], serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serialize_array(value, serializer)
            }

            pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; $size], D::Error>
            where
                D: Deserializer<'de>,
            {
                deserialize_array(deserializer)
            }
        }
    };
}

fixed_array_module!(fixed_12, 12);
fixed_array_module!(fixed_16, 16);
fixed_array_module!(fixed_32, 32);
