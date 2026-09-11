use serde::de::DeserializeOwned;
use serde::Serialize;

/// Trait providing dynamic, strongly-typed extension metadata for entities.
///
/// Other modules (e.g. `apich-git`, `apich-slide`, plugins) can attach arbitrary
/// domain structs to entities without requiring database schema alterations.
pub trait ExtensibleMetadata {
    fn metadata(&self) -> &serde_json::Value;
    fn metadata_mut(&mut self) -> &mut serde_json::Value;

    /// Retrieve strongly-typed extension data stored under `key`
    fn get_ext<T: DeserializeOwned>(
        &self,
        key: &str,
    ) -> Option<T> {
        self.metadata()
            .get(key)
            .and_then(|val| serde_json::from_value(val.clone()).ok())
    }

    /// Store strongly-typed extension data under `key`
    fn set_ext<T: Serialize>(
        &mut self,
        key: &str,
        value: T,
    ) -> Result<(), serde_json::Error> {
        let val = serde_json::to_value(value)?;
        if let Some(map) = self.metadata_mut().as_object_mut() {
            map.insert(key.to_string(), val);
        } else {
            let mut map = serde_json::Map::new();
            map.insert(key.to_string(), val);
            *self.metadata_mut() = serde_json::Value::Object(map);
        }
        Ok(())
    }
}
