//! String-only readers for the wire format's unit enum variants.
macro_rules! string_enum {
    ($name:ty, $($variant:ident),+ $(,)?) => {
        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self,D::Error> {
                let value = <String as serde::Deserialize>::deserialize(deserializer)?;
                match value.as_str() {
                    $(stringify!($variant)=>Ok(Self::$variant),)+
                    _=>Err(serde::de::Error::custom("invalid unit enum variant")),
                }
            }
        }
    };
}
pub(crate) use string_enum;
