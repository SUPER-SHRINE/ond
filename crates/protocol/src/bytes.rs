use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&STANDARD.encode(bytes))
}
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    STANDARD
        .decode(String::deserialize(deserializer)?)
        .map_err(serde::de::Error::custom)
}
