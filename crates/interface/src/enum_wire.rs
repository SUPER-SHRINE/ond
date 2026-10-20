//! Unit enum values use JSON strings; only data variants use tagged objects.
use crate::{Field, Kind, Signature, TypeRef, Value, ValueKind};
use serde::{Deserialize, Deserializer, de::Error};

impl<'de> Deserialize<'de> for Kind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        enum Payload {
            Pointer(TypeRef),
            Array { length: u32, element: TypeRef },
            Function(Signature),
            Struct(Vec<Field>),
            Defined(TypeRef),
        }
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Unit(String),
            Payload(Payload),
        }
        Ok(match Wire::deserialize(deserializer)? {
            Wire::Unit(name) => match name.as_str() {
                "Bool" => Kind::Bool,
                "U8" => Kind::U8,
                "I8" => Kind::I8,
                "U16" => Kind::U16,
                "I16" => Kind::I16,
                "U32" => Kind::U32,
                "I32" => Kind::I32,
                "F32" => Kind::F32,
                _ => return Err(D::Error::custom("invalid primitive type variant")),
            },
            Wire::Payload(payload) => match payload {
                Payload::Pointer(value) => Kind::Pointer(value),
                Payload::Array { length, element } => Kind::Array { length, element },
                Payload::Function(value) => Kind::Function(value),
                Payload::Struct(value) => Kind::Struct(value),
                Payload::Defined(value) => Kind::Defined(value),
            },
        })
    }
}

impl<'de> Deserialize<'de> for ValueKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        enum Payload {
            Integer(u64),
            Float32(u32),
            Bool(bool),
            Composite(Vec<(u32, Value)>),
        }
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Unit(String),
            Payload(Payload),
        }
        Ok(match Wire::deserialize(deserializer)? {
            Wire::Unit(name) => match name.as_str() {
                "Zero" => ValueKind::Zero,
                "Nil" => ValueKind::Nil,
                _ => return Err(D::Error::custom("invalid unit value variant")),
            },
            Wire::Payload(payload) => match payload {
                Payload::Integer(value) => ValueKind::Integer(value),
                Payload::Float32(value) => ValueKind::Float32(value),
                Payload::Bool(value) => ValueKind::Bool(value),
                Payload::Composite(value) => ValueKind::Composite(value),
            },
        })
    }
}
