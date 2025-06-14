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
                .as_millis() as u64
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
        // First check if key exists and if it's expired
        if let Some(store_value) = self.data.get(key) {
            if let Some(expiry_timestamp) = store_value.expiry {
                let current_timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_millis() as u64;

                if current_timestamp >= expiry_timestamp {
                    self.data.remove(key);
                    self.dirty = true;
                    return None;
                }
            }
            // Key exists and hasn't expired
            self.data.get_mut(key)
        } else {
            // Key doesn't exist
            None
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_new_storage_is_empty() {
        let mut storage = Storage::new();
        assert!(storage.get("some_key").is_none());
        assert!(!storage.is_dirty());
    }

    #[test]
    fn test_set_and_get_string_value() {
        let mut storage = Storage::new();
        let key = "hello".to_string();
        let value = "world".to_string();

        storage.set(key.clone(), value.clone(), None);
        assert!(storage.is_dirty());

        let stored_value = storage.get(&key).unwrap();
        match &stored_value.data {
            Value::String(s) => assert_eq!(s, &value),
            _ => panic!("Expected string value"),
        }
        assert!(stored_value.expiry.is_none());
    }

    #[test]
    fn test_set_with_expiry() {
        let mut storage = Storage::new();
        let key = "expiring_key".to_string();
        let value = "expiring_value".to_string();
        let expiry = Duration::from_secs(1);

        storage.set(key.clone(), value.clone(), Some(expiry));
        let stored_value = storage.get(&key).unwrap();
        
        match &stored_value.data {
            Value::String(s) => assert_eq!(s, &value),
            _ => panic!("Expected string value"),
        }
        assert!(stored_value.expiry.is_some());
    }

    #[test]
    fn test_set_overwrite_value() {
        let mut storage = Storage::new();
        let key = "test_key".to_string();
        let initial_value = "initial".to_string();
        let new_value = "overwritten".to_string();

        storage.set(key.clone(), initial_value.clone(), None);
        storage.set(key.clone(), new_value.clone(), None);

        let stored_value = storage.get(&key).unwrap();
        match &stored_value.data {
            Value::String(s) => assert_eq!(s, &new_value),
            _ => panic!("Expected string value"),
        }
    }

    #[test]
    fn test_get_non_existent_key() {
        let mut storage = Storage::new();
        assert!(storage.get("non_existent_key").is_none());
    }

    #[test]
    fn test_remove_key() {
        let mut storage = Storage::new();
        let key = "to_remove".to_string();
        let value = "some_value".to_string();

        storage.set(key.clone(), value.clone(), None);
        assert!(storage.get(&key).is_some());

        let removed_value = storage.remove(&key);
        assert!(removed_value.is_some());
        
        if let Some(store_value) = removed_value {
            match store_value.data {
                Value::String(s) => assert_eq!(s, value),
                _ => panic!("Expected string value"),
            }
        }

        assert!(storage.get(&key).is_none());
        assert!(storage.is_dirty());
    }

    #[test]
    fn test_remove_non_existent_key() {
        let mut storage = Storage::new();
        assert!(storage.remove("non_existent_key").is_none());
    }

    #[test]
    fn test_dirty_flag() {
        let mut storage = Storage::new();
        assert!(!storage.is_dirty());
        
        storage.set("key".to_string(), "value".to_string(), None);
        assert!(storage.is_dirty());
        
        storage.clear_dirty_flag();
        assert!(!storage.is_dirty());
    }

    // List operations tests
    #[test]
    fn test_lpush_new_list() {
        let mut storage = Storage::new();
        let key = "mylist";
        let values = vec!["value1".to_string(), "value2".to_string()];

        let result = storage.lpush(key, values.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
        assert!(storage.is_dirty());

        // Check that values were inserted in reverse order (left push)
        let range_result = storage.lrange(key, 0, -1).unwrap().unwrap();
        assert_eq!(range_result, vec!["value2", "value1"]);
    }

    #[test]
    fn test_rpush_new_list() {
        let mut storage = Storage::new();
        let key = "mylist";
        let values = vec!["value1".to_string(), "value2".to_string()];

        let result = storage.rpush(key, values.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);

        // Check that values were inserted in order (right push)
        let range_result = storage.lrange(key, 0, -1).unwrap().unwrap();
        assert_eq!(range_result, vec!["value1", "value2"]);
    }

    #[test]
    fn test_lpush_existing_list() {
        let mut storage = Storage::new();
        let key = "mylist";

        storage.rpush(key, vec!["existing".to_string()]).unwrap();
        let result = storage.lpush(key, vec!["new".to_string()]);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);

        let range_result = storage.lrange(key, 0, -1).unwrap().unwrap();
        assert_eq!(range_result, vec!["new", "existing"]);
    }

    #[test]
    fn test_lpush_wrong_type() {
        let mut storage = Storage::new();
        let key = "stringkey";
        
        storage.set(key.to_string(), "stringvalue".to_string(), None);
        let result = storage.lpush(key, vec!["value".to_string()]);
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "WRONGTYPE Operation against a key holding the wrong kind of value");
    }

    #[test]
    fn test_lrange_empty_list() {
        let mut storage = Storage::new();
        let result = storage.lrange("nonexistent", 0, -1);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_lrange_with_indices() {
        let mut storage = Storage::new();
        let key = "mylist";
        let values = vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()];
        
        storage.rpush(key, values).unwrap();

        // Test positive indices
        let result = storage.lrange(key, 1, 2).unwrap().unwrap();
        assert_eq!(result, vec!["b", "c"]);

        // Test negative indices
        let result = storage.lrange(key, -2, -1).unwrap().unwrap();
        assert_eq!(result, vec!["c", "d"]);

        // Test full range
        let result = storage.lrange(key, 0, -1).unwrap().unwrap();
        assert_eq!(result, vec!["a", "b", "c", "d"]);

        // Test out of range
        let result = storage.lrange(key, 10, 20).unwrap().unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_lrange_wrong_type() {
        let mut storage = Storage::new();
        let key = "stringkey";
        
        storage.set(key.to_string(), "stringvalue".to_string(), None);
        let result = storage.lrange(key, 0, -1);
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "WRONGTYPE Operation against a key holding the wrong kind of value");
    }

    // Hash operations tests
    #[test]
    fn test_hset_new_hash() {
        let mut storage = Storage::new();
        let key = "myhash".to_string();
        let field = "field1".to_string();
        let value = "value1".to_string();

        let result = storage.hset(key.clone(), field.clone(), value.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1); // New field
        assert!(storage.is_dirty());

        let get_result = storage.hget(&key, &field).unwrap();
        assert_eq!(get_result.unwrap(), &value);
    }

    #[test]
    fn test_hset_existing_field() {
        let mut storage = Storage::new();
        let key = "myhash".to_string();
        let field = "field1".to_string();

        storage.hset(key.clone(), field.clone(), "old_value".to_string()).unwrap();
        let result = storage.hset(key.clone(), field.clone(), "new_value".to_string());
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // Existing field

        let get_result = storage.hget(&key, &field).unwrap();
        assert_eq!(get_result.unwrap(), "new_value");
    }

    #[test]
    fn test_hset_wrong_type() {
        let mut storage = Storage::new();
        let key = "stringkey".to_string();
        
        storage.set(key.clone(), "stringvalue".to_string(), None);
        let result = storage.hset(key, "field".to_string(), "value".to_string());
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "WRONGTYPE Operation against a key holding the wrong kind of value");
    }

    #[test]
    fn test_hget_nonexistent_key() {
        let mut storage = Storage::new();
        let result = storage.hget("nonexistent", "field");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_hget_nonexistent_field() {
        let mut storage = Storage::new();
        let key = "myhash".to_string();
        
        storage.hset(key.clone(), "field1".to_string(), "value1".to_string()).unwrap();
        let result = storage.hget(&key, "nonexistent_field");
        
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_hdel() {
        let mut storage = Storage::new();
        let key = "myhash".to_string();
        
        storage.hset(key.clone(), "field1".to_string(), "value1".to_string()).unwrap();
        storage.hset(key.clone(), "field2".to_string(), "value2".to_string()).unwrap();
        storage.hset(key.clone(), "field3".to_string(), "value3".to_string()).unwrap();

        // Delete existing fields
        let result = storage.hdel(&key, vec!["field1".to_string(), "field3".to_string()]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);

        // Check that field2 still exists
        assert!(storage.hget(&key, "field2").unwrap().is_some());
        assert!(storage.hget(&key, "field1").unwrap().is_none());
        assert!(storage.hget(&key, "field3").unwrap().is_none());

        // Delete non-existent field
        let result = storage.hdel(&key, vec!["nonexistent".to_string()]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_hdel_nonexistent_key() {
        let mut storage = Storage::new();
        let result = storage.hdel("nonexistent", vec!["field".to_string()]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_hlen() {
        let mut storage = Storage::new();
        let key = "myhash".to_string();

        // Empty/nonexistent hash
        let result = storage.hlen(&key);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // Add some fields
        storage.hset(key.clone(), "field1".to_string(), "value1".to_string()).unwrap();
        storage.hset(key.clone(), "field2".to_string(), "value2".to_string()).unwrap();

        let result = storage.hlen(&key);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);

        // Delete a field
        storage.hdel(&key, vec!["field1".to_string()]).unwrap();
        let result = storage.hlen(&key);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_hlen_wrong_type() {
        let mut storage = Storage::new();
        let key = "stringkey".to_string();
        
        storage.set(key.clone(), "stringvalue".to_string(), None);
        let result = storage.hlen(&key);
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "WRONGTYPE Operation against a key holding the wrong kind of value");
    }

    #[test]
    fn test_hgetall() {
        let mut storage = Storage::new();
        let key = "myhash".to_string();

        // Empty/nonexistent hash
        let result = storage.hgetall(&key);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Add some fields
        storage.hset(key.clone(), "field1".to_string(), "value1".to_string()).unwrap();
        storage.hset(key.clone(), "field2".to_string(), "value2".to_string()).unwrap();

        let result = storage.hgetall(&key).unwrap().unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result.get("field1").unwrap(), "value1");
        assert_eq!(result.get("field2").unwrap(), "value2");
    }

    #[test]
    fn test_hgetall_wrong_type() {
        let mut storage = Storage::new();
        let key = "stringkey".to_string();
        
        storage.set(key.clone(), "stringvalue".to_string(), None);
        let result = storage.hgetall(&key);
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "WRONGTYPE Operation against a key holding the wrong kind of value");
    }

    #[test]
    fn test_serialization() {
        let mut storage = Storage::new();
        
        // Add some data
        storage.set("string_key".to_string(), "string_value".to_string(), None);
        storage.rpush("list_key", vec!["item1".to_string(), "item2".to_string()]).unwrap();
        storage.hset("hash_key".to_string(), "field1".to_string(), "value1".to_string()).unwrap();

        // Serialize
        let serialized = bincode::serialize(&storage).unwrap();
        
        // Deserialize
        let mut deserialized: Storage = bincode::deserialize(&serialized).unwrap();
        
        // Verify data integrity
        let string_val = deserialized.get("string_key").unwrap();
        match &string_val.data {
            Value::String(s) => assert_eq!(s, "string_value"),
            _ => panic!("Expected string value"),
        }

        let list_val = deserialized.lrange("list_key", 0, -1).unwrap().unwrap();
        assert_eq!(list_val, vec!["item1", "item2"]);

        let hash_val = deserialized.hget("hash_key", "field1").unwrap().unwrap();
        assert_eq!(hash_val, "value1");

        // Dirty flag should be reset after deserialization
        assert!(!deserialized.is_dirty());
    }
}
