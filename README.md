# IronCache - A Redis-like In-Memory Database

IronCache is a learning project to build a Redis-like in-memory, persistent key-value store from scratch in Rust. It features a concurrent TCP server built with Tokio that can handle multiple clients simultaneously.

This project is built to explore concepts in network programming, concurrency, data structures, and persistence strategies, mimicking some of the core functionalities of Redis.

---

## Features

* **Concurrent TCP Server**: Built on Tokio, capable of handling multiple client connections at once.
* **Multi-Data Type Support**:
    * **Strings**: Simple key-value pairs.
    * **Lists**: Ordered collections of strings, supporting `LPUSH` and `RPUSH`.
    * **Hashes**: Store objects as maps of field-value pairs.
* **Key Expiry**: Set a Time-To-Live (TTL) on keys using the `EX` option with the `SET` command. Expired keys are removed on access (lazy eviction).
* **Data Persistence**:
    * **Snapshotting**: The entire database state can be saved to a `dump.db` file.
    * **Periodic Saving**: Automatically saves a snapshot to disk every 10 seconds if the data has changed.
    * **Manual Saving**: Force a snapshot at any time with the `SAVE` command.
    * **Recovery**: Automatically loads data from `dump.db` on startup.

---

## How to Run

1.  **Prerequisites**: Ensure you have the Rust toolchain installed. You can get it from [rustup.rs](https://rustup.rs/).
2.  **Clone & Build**:
    ```bash
    # (Assuming you have the project locally)
    # Build the project in release mode for better performance
    cargo build --release
    ```
3.  **Run the Server**:
    ```bash
    # Run the compiled binary
    ./target/release/iron-cache
    # The server will start and print:
    # Server is running on port 6969
    ```

---

## How to Interact

You can connect to the server using a simple network utility like `netcat`. The server uses a simple, space-separated text protocol.

**Open a new terminal** and connect to the server:

```bash
netcat 127.0.0.1 6969