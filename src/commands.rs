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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_parse_set_basic() {
        let input = b"SET mykey myvalue";
        let result = Command::parse(input).unwrap();

        match result {
            Command::Set { key, value, expiry } => {
                assert_eq!(key, "mykey");
                assert_eq!(value, "myvalue");
                assert!(expiry.is_none());
            }
            _ => panic!("Expected SET command"),
        }
    }

    #[test]
    fn test_parse_set_with_expiry() {
        let input = b"SET mykey myvalue EX 60";
        let result = Command::parse(input).unwrap();

        match result {
            Command::Set { key, value, expiry } => {
                assert_eq!(key, "mykey");
                assert_eq!(value, "myvalue");
                assert_eq!(expiry.unwrap(), Duration::from_secs(60));
            }
            _ => panic!("Expected SET command with expiry"),
        }
    }

    #[test]
    fn test_parse_set_case_insensitive() {
        let input = b"set mykey myvalue ex 30";
        let result = Command::parse(input).unwrap();

        match result {
            Command::Set { key, value, expiry } => {
                assert_eq!(key, "mykey");
                assert_eq!(value, "myvalue");
                assert_eq!(expiry.unwrap(), Duration::from_secs(30));
            }
            _ => panic!("Expected SET command"),
        }
    }

    #[test]
    fn test_parse_set_invalid_expiry() {
        let input = b"SET mykey myvalue EX invalid";
        let result = Command::parse(input);

        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::InvalidArgument(msg) => {
                assert!(msg.contains("Expiry time must be a positive integer"));
            }
            _ => panic!("Expected InvalidArgument error"),
        }
    }

    #[test]
    fn test_parse_set_missing_args() {
        let input = b"SET";
        let result = Command::parse(input);
        assert!(result.is_err());

        let input = b"SET mykey";
        let result = Command::parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_get() {
        let input = b"GET mykey";
        let result = Command::parse(input).unwrap();

        match result {
            Command::Get { key } => {
                assert_eq!(key, "mykey");
            }
            _ => panic!("Expected GET command"),
        }
    }

    #[test]
    fn test_parse_get_case_insensitive() {
        let input = b"get mykey";
        let result = Command::parse(input).unwrap();

        match result {
            Command::Get { key } => {
                assert_eq!(key, "mykey");
            }
            _ => panic!("Expected GET command"),
        }
    }

    #[test]
    fn test_parse_del() {
        let input = b"DEL mykey";
        let result = Command::parse(input).unwrap();

        match result {
            Command::Del { key } => {
                assert_eq!(key, "mykey");
            }
            _ => panic!("Expected DEL command"),
        }
    }

    #[test]
    fn test_parse_lpush() {
        let input = b"LPUSH mylist value1 value2 value3";
        let result = Command::parse(input).unwrap();

        match result {
            Command::LPush { key, values } => {
                assert_eq!(key, "mylist");
                assert_eq!(values, vec!["value1", "value2", "value3"]);
            }
            _ => panic!("Expected LPUSH command"),
        }
    }

    #[test]
    fn test_parse_lpush_single_value() {
        let input = b"LPUSH mylist single_value";
        let result = Command::parse(input).unwrap();

        match result {
            Command::LPush { key, values } => {
                assert_eq!(key, "mylist");
                assert_eq!(values, vec!["single_value"]);
            }
            _ => panic!("Expected LPUSH command"),
        }
    }

    #[test]
    fn test_parse_lpush_no_values() {
        let input = b"LPUSH mylist";
        let result = Command::parse(input);

        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::InvalidArgument(msg) => {
                assert!(msg.contains("LPUSH <key> <value> [value ...]"));
            }
            _ => panic!("Expected InvalidArgument error"),
        }
    }

    #[test]
    fn test_parse_rpush() {
        let input = b"RPUSH mylist value1 value2";
        let result = Command::parse(input).unwrap();

        match result {
            Command::RPush { key, values } => {
                assert_eq!(key, "mylist");
                assert_eq!(values, vec!["value1", "value2"]);
            }
            _ => panic!("Expected RPUSH command"),
        }
    }

    #[test]
    fn test_parse_lrange() {
        let input = b"LRANGE mylist 0 -1";
        let result = Command::parse(input).unwrap();

        match result {
            Command::LRange { key, start, stop } => {
                assert_eq!(key, "mylist");
                assert_eq!(start, 0);
                assert_eq!(stop, -1);
            }
            _ => panic!("Expected LRANGE command"),
        }
    }

    #[test]
    fn test_parse_lrange_positive_indices() {
        let input = b"LRANGE mylist 1 3";
        let result = Command::parse(input).unwrap();

        match result {
            Command::LRange { key, start, stop } => {
                assert_eq!(key, "mylist");
                assert_eq!(start, 1);
                assert_eq!(stop, 3);
            }
            _ => panic!("Expected LRANGE command"),
        }
    }

    #[test]
    fn test_parse_lrange_invalid_indices() {
        let input = b"LRANGE mylist abc def";
        let result = Command::parse(input);
        assert!(result.is_err());

        let input = b"LRANGE mylist 0 abc";
        let result = Command::parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_hset() {
        let input = b"HSET myhash field1 value1";
        let result = Command::parse(input).unwrap();

        match result {
            Command::HSet { key, field, value } => {
                assert_eq!(key, "myhash");
                assert_eq!(field, "field1");
                assert_eq!(value, "value1");
            }
            _ => panic!("Expected HSET command"),
        }
    }

    #[test]
    fn test_parse_hget() {
        let input = b"HGET myhash field1";
        let result = Command::parse(input).unwrap();

        match result {
            Command::HGet { key, field } => {
                assert_eq!(key, "myhash");
                assert_eq!(field, "field1");
            }
            _ => panic!("Expected HGET command"),
        }
    }

    #[test]
    fn test_parse_hdel() {
        let input = b"HDEL myhash field1 field2 field3";
        let result = Command::parse(input).unwrap();

        match result {
            Command::HDel { key, fields } => {
                assert_eq!(key, "myhash");
                assert_eq!(fields, vec!["field1", "field2", "field3"]);
            }
            _ => panic!("Expected HDEL command"),
        }
    }

    #[test]
    fn test_parse_hdel_single_field() {
        let input = b"HDEL myhash field1";
        let result = Command::parse(input).unwrap();

        match result {
            Command::HDel { key, fields } => {
                assert_eq!(key, "myhash");
                assert_eq!(fields, vec!["field1"]);
            }
            _ => panic!("Expected HDEL command"),
        }
    }

    #[test]
    fn test_parse_hlen() {
        let input = b"HLEN myhash";
        let result = Command::parse(input).unwrap();

        match result {
            Command::HLen { key } => {
                assert_eq!(key, "myhash");
            }
            _ => panic!("Expected HLEN command"),
        }
    }

    #[test]
    fn test_parse_hgetall() {
        let input = b"HGETALL myhash";
        let result = Command::parse(input).unwrap();

        match result {
            Command::HGetAll { key } => {
                assert_eq!(key, "myhash");
            }
            _ => panic!("Expected HGETALL command"),
        }
    }

    #[test]
    fn test_parse_save() {
        let input = b"SAVE";
        let result = Command::parse(input).unwrap();

        match result {
            Command::Save => {},
            _ => panic!("Expected SAVE command"),
        }
    }

    #[test]
    fn test_parse_save_case_insensitive() {
        let input = b"save";
        let result = Command::parse(input).unwrap();

        match result {
            Command::Save => {},
            _ => panic!("Expected SAVE command"),
        }
    }

    #[test]
    fn test_parse_unknown_command() {
        let input = b"UNKNOWN command";
        let result = Command::parse(input);

        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::UnknownCommand => {},
            _ => panic!("Expected UnknownCommand error"),
        }
    }

    #[test]
    fn test_parse_empty_input() {
        let input = b"";
        let result = Command::parse(input);

        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::UnknownCommand => {},
            _ => panic!("Expected UnknownCommand error"),
        }
    }

    #[test]
    fn test_parse_whitespace_only() {
        let input = b"   ";
        let result = Command::parse(input);

        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::UnknownCommand => {},
            _ => panic!("Expected UnknownCommand error"),
        }
    }

    #[test]
    fn test_parse_with_extra_whitespace() {
        let input = b"  SET   mykey   myvalue  ";
        let result = Command::parse(input).unwrap();

        match result {
            Command::Set { key, value, expiry } => {
                assert_eq!(key, "mykey");
                assert_eq!(value, "myvalue");
                assert!(expiry.is_none());
            }
            _ => panic!("Expected SET command"),
        }
    }

    #[test]
    fn test_parse_hlen_with_extra_args() {
        let input = b"HLEN myhash extra_arg";
        let result = Command::parse(input);

        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::InvalidArgument(msg) => {
                assert!(msg.contains("Usage: HLEN <key>"));
            }
            _ => panic!("Expected InvalidArgument error"),
        }
    }

    #[test]
    fn test_command_debug_trait() {
        let cmd = Command::Set {
            key: "testkey".to_string(),
            value: "testvalue".to_string(),
            expiry: Some(Duration::from_secs(30)),
        };

        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Set"));
        assert!(debug_str.contains("testkey"));
        assert!(debug_str.contains("testvalue"));
    }

    #[test]
    fn test_parse_error_debug_trait() {
        let error = ParseError::InvalidArgument("Test error message".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("InvalidArgument"));
        assert!(debug_str.contains("Test error message"));
    }
}
