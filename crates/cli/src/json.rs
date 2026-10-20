//! Shared canonical writer for cache files and public CLI JSON.
pub(crate) fn encode<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, Vec<String>> {
    let value = serde_json::to_value(value).map_err(|error| vec![error.to_string()])?;
    serde_json::to_vec(&value).map_err(|error| vec![error.to_string()])
}
