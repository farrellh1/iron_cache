use std::time::Duration;

#[derive(Debug)]
pub enum ParseError {
    UnknownCommand,
    InvalidArgument(String), // Can hold a message about what went wrong
}

#[derive(Debug)]
pub enum Command {
    Get {
        key: String,
    },
    Set {
        key: String,
        value: String,
        expiry: Option<Duration>,
    },
    Del {
        key: String,
    },
    LPush {
        key: String,
        values: Vec<String>,
    },
    RPush {
        key: String,
        values: Vec<String>,
    },
    LRange {
        key: String,
        start: i64,
        stop: i64,
    },
    HSet {
        key: String,
        field: String,
        value: String,
    },
    HGet {
        key: String,
        field: String,
    },
    HDel {
        key: String,
        fields: Vec<String>,
    },
    HLen {
        key: String,
    },
    HGetAll {
        key: String,
    },
    Save,
}

#[derive(Debug)]
pub enum CommandParseError {
    InvalidCommand(String),
    WrongType(String),
    WrongNumberOfArgs,
}

impl Command {
    pub fn parse(buffer: &[u8]) -> Result<Command, ParseError> {
        let parts = std::str::from_utf8(buffer)
            .unwrap_or("")
            .split_whitespace()
            .collect::<Vec<&str>>();

        match parts.as_slice() {
            ["SET" | "set", key, value, "EX" | "ex", seconds] => {
                let seconds = seconds.parse::<u64>().map_err(|_| {
                    ParseError::InvalidArgument(
                        "Expiry time must be a positive integer.".to_string(),
                    )
                })?;

                Ok(Command::Set {
                    key: key.to_string(),
                    value: value.to_string(),
                    expiry: Some(Duration::from_secs(seconds)),
                })
            }
            ["SET" | "set", key, value] => Ok(Command::Set {
                key: key.to_string(),
                value: value.to_string(),
                expiry: None,
            }),

            ["GET" | "get", key] => Ok(Command::Get {
                key: key.to_string(),
            }),
            ["DEL" | "del", key] => Ok(Command::Del {
                key: key.to_string(),
            }),
            ["LPUSH" | "lpush", key, values @ ..] => {
                if values.is_empty() {
                    return Err(ParseError::InvalidArgument(
                        "Usage: LPUSH <key> <value> [value ...]".to_string(),
                    ));
                }

                Ok(Command::LPush {
                    key: key.to_string(),
                    values: values.iter().map(|s| s.to_string()).collect(),
                })
            }
            ["RPUSH" | "rpush", key, values @ ..] => {
                if values.is_empty() {
                    return Err(ParseError::InvalidArgument(
                        "Usage: RPUSH <key> <value> [value ...]".to_string(),
                    ));
                }

                Ok(Command::RPush {
                    key: key.to_string(),
                    values: values.iter().map(|s| s.to_string()).collect(),
                })
            }
            ["LRANGE" | "lrange", key, start, stop] => {
                let start = start.parse::<i64>().map_err(|_| {
                    ParseError::InvalidArgument("start index must be an integer.".to_string())
                })?;
                let stop = stop.parse::<i64>().map_err(|_| {
                    ParseError::InvalidArgument("stop index must be an integer.".to_string())
                })?;

                Ok(Command::LRange {
                    key: key.to_string(),
                    start,
                    stop,
                })
            }
            ["HSET" | "hset", key, field, value] => Ok(Command::HSet {
                key: key.to_string(),
                field: field.to_string(),
                value: value.to_string(),
            }),
            ["HGET" | "hget", key, field] => Ok(Command::HGet {
                key: key.to_string(),
                field: field.to_string(),
            }),
            ["HDEL" | "hdel", key, fields @ ..] if !fields.is_empty() => Ok(Command::HDel {
                key: key.to_string(),
                fields: fields.iter().map(|s| s.to_string()).collect(),
            }),
            ["HLEN" | "hlen", key] => Ok(Command::HLen {
                key: key.to_string(),
            }),
            ["HGETALL" | "hgetall", key] => Ok(Command::HGetAll {
                key: key.to_string(),
            }),
            ["SET" | "set"] => Err(ParseError::InvalidArgument(
                "SET command requires both key and value. Usage: SET <key> <value> [EX <seconds>]"
                    .to_string(),
            )),
            ["SET" | "set", _] => Err(ParseError::InvalidArgument(
                "SET command requires both key and value. Usage: SET <key> <value> [EX <seconds>]"
                    .to_string(),
            )),
            ["SET" | "set", ..] => Err(ParseError::InvalidArgument(
                "Invalid SET command format. Usage: SET <key> <value> [EX <seconds>]".to_string(),
            )),
            ["GET" | "get", ..] | ["DEL" | "del", ..] => Err(ParseError::InvalidArgument(
                "Usage: GET|DEL <key>".to_string(),
            )),
            ["LRANGE" | "lrange", ..] => Err(ParseError::InvalidArgument(
                "Usage: LRANGE <key> <start> <stop>".to_string(),
            )),
            ["HSET" | "hset", ..] => Err(ParseError::InvalidArgument(
                "Usage: HSET <key> <field> <value>".to_string(),
            )),
            ["HGET" | "hget", ..] | ["HGETALL" | "hgetall", ..] => Err(
                ParseError::InvalidArgument("Usage: HGET|HGETALL <key> [field]".to_string()),
            ),
            ["HDEL" | "hdel", ..] => Err(ParseError::InvalidArgument(
                "Usage: HDEL <key> <field> [field ...]".to_string(),
            )),
            ["HLEN" | "hlen", _key, ..] => {
                Err(ParseError::InvalidArgument("Usage: HLEN <key>".to_string()))
            }
            ["SAVE" | "save"] => Ok(Command::Save),
            // Any other command is unknown
            _ => Err(ParseError::UnknownCommand),
        }
    }
}
