use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Serialize, Deserialize)]
pub enum Value {
    String(String),
    List(VecDeque<String>),
    Hash(HashMap<String, String>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StoreValue {
    pub data: Value,
    pub expiry: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Storage {
    data: HashMap<String, StoreValue>,
    // This field is used to track if the storage has been modified.
    #[serde(skip)]
    dirty: bool,
}

impl Storage {
    pub fn new() -> Self {
        Storage {
            data: HashMap::new(),
            dirty: false,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty_flag(&mut self) {
        self.dirty = false;
    }

    pub fn set(&mut self, key: String, value: String, expiry: Option<Duration>) {
        let expiry_timestamp = expiry.map(|duration| {
            let future_time = SystemTime::now() + duration;
            future_time
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs()
        });

        self.data.insert(
            key,
            StoreValue {
                data: Value::String(value),
                expiry: expiry_timestamp,
            },
        );
        self.dirty = true;
    }

    pub fn get(&mut self, key: &str) -> Option<&mut StoreValue> {
        if let Some(store_value) = self.data.get(key) {
            if let Some(expiry_timestamp) = store_value.expiry {
                let current_timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs();

                if current_timestamp >= expiry_timestamp {
                    self.data.remove(key);
                    self.dirty = true;
                    return None;
                }
            }
        }
        self.data.get_mut(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<StoreValue> {
        // Return the inner data string when removing.
        let result = self.data.remove(key);
        if result.is_some() {
            self.dirty = true;
        }

        result
    }

    pub fn lpush(&mut self, key: &str, values: Vec<String>) -> Result<usize, &'static str> {
        let entry = self
            .data
            .entry(key.to_string())
            .or_insert_with(|| StoreValue {
                data: Value::List(VecDeque::new()),
                expiry: None,
            });

        match &mut entry.data {
            Value::List(list) => {
                for v in values.into_iter() {
                    list.push_front(v);
                }
                self.dirty = true;
                Ok(list.len())
            }
            _ => Err("WRONGTYPE Operation against a key holding the wrong kind of value"),
        }
    }

    pub fn rpush(&mut self, key: &str, values: Vec<String>) -> Result<usize, &'static str> {
        let entry = self
            .data
            .entry(key.to_string())
            .or_insert_with(|| StoreValue {
                data: Value::List(VecDeque::new()),
                expiry: None,
            });

        match &mut entry.data {
            Value::List(list) => {
                for v in values {
                    list.push_back(v);
                }
                self.dirty = true;
                Ok(list.len())
            }
            _ => Err("WRONGTYPE Operation against a key holding the wrong kind of value"),
        }
    }

    pub fn lrange(
        &mut self,
        key: &str,
        start: i64,
        stop: i64,
    ) -> Result<Option<Vec<String>>, &'static str> {
        match self.data.get(key) {
            None => Ok(None),
            Some(store_value) => match &store_value.data {
                Value::List(list) => {
                    let len = list.len() as i64;

                    // Convert Redis-style negative indices to regular indices
                    let start = if start < 0 { len + start } else { start }.max(0) as usize;
                    let stop = if stop < 0 { len + stop } else { stop }.max(0) as usize;

                    if start >= list.len() || start > stop {
                        return Ok(Some(Vec::new())); // Return empty list for out of range
                    }

                    // `..=stop` is inclusive. Ensure `stop` is within bounds.
                    let stop = stop.min(list.len() - 1);

                    let result = list
                        .iter()
                        .skip(start)
                        .take(stop - start + 1)
                        .cloned()
                        .collect();
                    Ok(Some(result))
                }
                _ => Err("WRONGTYPE Operation against a key holding the wrong kind of value"),
            },
        }
    }

    pub fn hset(&mut self, key: String, field: String, value: String) -> Result<i32, &'static str> {
        let entry = self
            .data
            .entry(key.to_string())
            .or_insert_with(|| StoreValue {
                data: Value::Hash(HashMap::new()),
                expiry: None,
            });

        match &mut entry.data {
            Value::Hash(hash) => {
                let result = if hash.contains_key(&field) { 0 } else { 1 };
                hash.insert(field, value);
                self.dirty = true;
                Ok(result)
            }
            _ => Err("WRONGTYPE Operation against a key holding the wrong kind of value"),
        }
    }

    pub fn hget(&mut self, key: &str, field: &str) -> Result<Option<&String>, &'static str> {
        match self.data.get(key) {
            None => Ok(None),
            Some(store_value) => match &store_value.data {
                Value::Hash(hash) => Ok(hash.get(field)),
                _ => Err("WRONGTYPE Operation against a key holding the wrong kind of value"),
            },
        }
    }

    pub fn hdel(&mut self, key: &str, fields: Vec<String>) -> Result<i32, &'static str> {
        match self.data.get_mut(key) {
            None => Ok(0),
            Some(store_value) => match &mut store_value.data {
                Value::Hash(hash) => {
                    let mut deleted_count = 0;
                    for field in fields {
                        if hash.remove(&field).is_some() {
                            deleted_count += 1;
                        }
                    }
                    Ok(deleted_count)
                }
                _ => Err("WRONGTYPE Operation against a key holding the wrong kind of value"),
            },
        }
    }

    pub fn hlen(&mut self, key: &str) -> Result<usize, &'static str> {
        match self.data.get(key) {
            None => Ok(0),
            Some(store_value) => match &store_value.data {
                Value::Hash(map) => Ok(map.len()),
                _ => Err("WRONGTYPE Operation against a key holding the wrong kind of value"),
            },
        }
    }

    pub fn hgetall(&self, key: &str) -> Result<Option<&HashMap<String, String>>, &'static str> {
        match self.data.get(key) {
            None => Ok(None),
            Some(store_value) => match &store_value.data {
                Value::Hash(hash) => Ok(Some(hash)),
                _ => Err("WRONGTYPE Operation against a key holding the wrong kind of value"),
            },
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_new_storage_is_empty() {
//         let storage = Storage::new();
//         // Test that getting a non-existent key returns None
//         assert!(storage.get("some_key").is_none());
//     }

//     #[test]
//     fn test_set_and_get_value() {
//         let mut storage = Storage::new();
//         let key = "hello".to_string();
//         let value = "world".to_string();

//         // Set a key-value pair
//         storage.set(key.clone(), value.clone());

//         // Get the value and assert it's correct
//         assert_eq!(storage.get(&key), Some(&value));
//     }

//     #[test]
//     fn test_set_overwrite_value() {
//         let mut storage = Storage::new();
//         let key = "test_key".to_string();
//         let initial_value = "initial".to_string();
//         let new_value = "overwritten".to_string();

//         storage.set(key.clone(), initial_value.clone());
//         assert_eq!(storage.get(&key), Some(&initial_value));

//         // Set the same key with a new value
//         storage.set(key.clone(), new_value.clone());
//         assert_eq!(storage.get(&key), Some(&new_value));
//     }

//     #[test]
//     fn test_get_non_existent_key() {
//         let storage = Storage::new();
//         assert_eq!(storage.get("non_existent_key"), None);
//     }

//     #[test]
//     fn test_remove_key() {
//         let mut storage = Storage::new();
//         let key = "to_remove".to_string();
//         let value = "some_value".to_string();

//         storage.set(key.clone(), value.clone());
//         assert_eq!(storage.get(&key), Some(&value)); // Ensure it's there

//         // Remove the key
//         let removed_value = storage.remove(&key);
//         assert_eq!(removed_value, Some(value)); // Check if correct value was returned

//         // Assert the key is no longer present
//         assert!(storage.get(&key).is_none());
//     }

//     #[test]
//     fn test_remove_non_existent_key() {
//         let mut storage = Storage::new();
//         assert_eq!(storage.remove("non_existent_key"), None);
//     }
// }
