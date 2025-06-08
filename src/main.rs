use iron_cache::commands::Command;
use iron_cache::storage::{Storage, Value};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

// Type alias for our shared database type for cleaner code
type Db = Arc<Mutex<Storage>>;
const DB_PATH: &str = "dump.db";
const SAVE_INTERVAL_SECS: u64 = 10;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:6969").await?;
    println!("Server is running on port 6969");

    let storage = match File::open(DB_PATH) {
        Ok(file) => {
            // Load existing data from the file
            let reader = BufReader::new(file);
            match bincode::deserialize_from(reader) {
                Ok(decoded) => {
                    println!("Loaded database from {}", DB_PATH);
                    decoded
                }
                Err(e) => {
                    eprintln!("Failed to deserialize storage: {}", e);
                    Storage::new() // Fallback to a new storage instance
                }
            }
        }
        Err(_) => {
            // If the file doesn't exist, create a new storage instance
            Storage::new()
        }
    };

    let db = Arc::new(Mutex::new(storage));

    let db_for_saving = db.clone();
    tokio::spawn(async move {
        loop {
            // Wait for the 10 seconds before saving the snapshot
            tokio::time::sleep(Duration::from_secs(SAVE_INTERVAL_SECS)).await;

            // Call the save function
            save_snapshot(&db_for_saving).await;
        }
    });

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("New connection from {}", addr);

        let db_clone = db.clone();

        tokio::spawn(async move {
            if let Err(e) = process_connection(socket, db_clone).await {
                eprintln!("Error processing connection from {}: {}", addr, e);
            }
        });
    }
}

/// Handles the entire lifecycle of a single client connection.
async fn process_connection(mut socket: TcpStream, db: Db) -> std::io::Result<()> {
    let mut buffer = [0; 1024];

    loop {
        match socket.read(&mut buffer).await {
            Ok(0) => return Ok(()), // Connection closed gracefully
            Ok(n) => {
                let response = match Command::parse(&buffer[..n]) {
                    Ok(command) => execute_command(command, &db).await,
                    Err(e) => format!("(error) {:?}\n", e),
                };

                // Write the response back to the client
                socket.write_all(response.as_bytes()).await?;
            }
            Err(e) => return Err(e), // Connection error
        }
    }
}

/// Executes a parsed command against the database.
async fn execute_command(command: Command, db: &Db) -> String {
    // Lock the mutex to get access to the storage
    let mut db_lock = db.lock().await;

    match command {
        Command::Set { key, value, expiry } => {
            db_lock.set(key, value, expiry);

            "OK\n".to_string()
        }
        Command::Get { key } => match db_lock.get(&key) {
            Some(store_value) => match &store_value.data {
                Value::String(s) => format!("{}\n", s),
                _ => "(error) WRONGTYPE Operation against a key holding the wrong kind of value\n"
                    .to_string(),
            },
            None => "NIL\n".to_string(),
        },
        Command::Del { key } => {
            db_lock.remove(&key);

            "OK\n".to_string()
        }
        Command::LPush { key, values } => match db_lock.lpush(&key, values) {
            Ok(len) => format!("(integer) {}\n", len),
            Err(msg) => format!("(error) {}\n", msg),
        },
        Command::RPush { key, values } => match db_lock.rpush(&key, values) {
            Ok(len) => format!("(integer) {}\n", len),
            Err(msg) => format!("(error) {}\n", msg),
        },
        Command::LRange { key, start, stop } => match db_lock.lrange(&key, start, stop) {
            Ok(Some(items)) => items
                .iter()
                .map(|item| format!("{}\n", item))
                .collect::<String>(),
            Ok(None) => "*(empty list)\n".to_string(),
            Err(msg) => format!("(error) {}\n", msg),
        },
        Command::HSet { key, field, value } => match db_lock.hset(key, field, value) {
            Ok(num) => format!("(integer) {}\n", num),
            Err(msg) => format!("(error) {}\n", msg),
        },
        Command::HGet { key, field } => match db_lock.hget(&key, &field) {
            Ok(Some(value)) => format!("{}\n", value),
            Ok(None) => "NIL\n".to_string(),
            Err(msg) => format!("(error) {}\n", msg),
        },
        Command::HDel { key, fields } => match db_lock.hdel(&key, fields) {
            Ok(num) => format!("(integer) {}\n", num),
            Err(msg) => format!("(error) {}\n", msg),
        },
        Command::HLen { key } => match db_lock.hlen(&key) {
            Ok(num) => format!("(integer) {}\n", num),
            Err(msg) => format!("(error) {}\n", msg),
        },
        Command::HGetAll { key } => match db_lock.hgetall(&key) {
            Ok(Some(hash)) => hash
                .iter()
                .map(|(k, v)| format!("{}: {}\n", k, v))
                .collect::<String>(),
            Ok(None) => "*(empty list)\n".to_string(),
            Err(msg) => format!("(error) {}\n", msg),
        },
        Command::Save => {
            // Save the snapshot of the database to disk
            save_snapshot(&db).await;

            "OK\n".to_string()
        }
    }
}

/// Saves a snapshot of the database to disk.
async fn save_snapshot(db: &Db) {
    // We lock the DB here to ensure a consistent state while saving.
    println!("Saving database snapshot...");

    let mut db_lock = db.lock().await;
    if !db_lock.is_dirty() {
        println!("No changes detected, skipping save.");
        return; // No changes to save
    }

    let db_clone_for_saving = Arc::clone(db);
    let path = DB_PATH.to_string();

    db_lock.clear_dirty_flag();

    drop(db_lock);
    let handle = tokio::task::spawn_blocking(move || {
        // We must lock the mutex here inside the synchronous context.
        let db_lock = db_clone_for_saving.blocking_lock();
        let file = File::create(path).expect("Failed to create db file");
        bincode::serialize_into(file, &*db_lock).expect("Failed to serialize db");
    });

    // Wait for the saving to complete.
    if let Err(e) = handle.await {
        eprintln!("Error saving snapshot: {}", e);
    } else {
        println!("Database snapshot saved successfully.");
    }
}
