use serde::{ Deserialize, Serialize };
use serde_json::json;
use std::{ collections::HashMap, fmt::Display };

use crate::messages::bot_command::BotCommand;

#[derive(Serialize, Deserialize, Debug)]
struct EmotePosition {
    start_position: String,
    end_position: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PrivateMessageRequest {
    pub tags: Option<HashMap<String, serde_json::Value>>,
    source: Option<Source>,
    pub command: Option<Command>,
    pub parameters: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Source {
    nick: Option<String>,
    host: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Command {
    pub command: String,
    pub channel: Option<String>,
    is_cap_request_enabled: Option<bool>,
    pub bot_command: Option<BotCommand>,
}

impl PrivateMessageRequest {
    pub fn new(raw_message: &str) -> PrivateMessageRequest {
        let mut idx = 0;

        // Parse tags
        let mut tags: Option<HashMap<String, serde_json::Value>> = None;
        if raw_message.chars().nth(idx) == Some('@') {
            if let Some(end_idx) = raw_message.find(' ') {
                let raw_tags_component = &raw_message[1..end_idx];
                tags = Some(parse_tags(raw_tags_component));
                idx = end_idx + 1;
            }
        }

        // Parse source
        let mut source: Option<Source> = None;
        if raw_message[idx..].starts_with(':') {
            if let Some(end_idx) = raw_message[idx..].find(' ') {
                let raw_source_component = &raw_message[idx + 1..idx + end_idx];
                source = Some(parse_source(raw_source_component));
                idx += end_idx + 1;
            }
        }

        // Parse command
        let command_start = idx;
        let mut command_end = raw_message.len();
        if let Some(colon_idx) = raw_message[idx..].find(':') {
            command_end = colon_idx + idx;
        }

        let raw_command_component = &raw_message[command_start..command_end].trim();

        let command = parse_command(raw_command_component);

        // Parse parameters
        let parameters = if command_end < raw_message.len() {
            Some(raw_message[command_end + 1..].to_string())
        } else {
            None
        };

        // Parse bot command if parameters exist and start with '!'
        if let Some(ref params) = parameters {
            if params.starts_with('!') {
                let command = command.map(|mut cmd| {
                    let (bot_command, bot_command_params) = parse_parameters(params);
                    cmd.bot_command = Some(BotCommand {
                        command: bot_command,
                        command_params: bot_command_params,
                    });
                    cmd
                });
                return PrivateMessageRequest {
                    tags,
                    source,
                    command,
                    parameters: Some(params.clone()),
                };
            }
        }

        PrivateMessageRequest {
            tags,
            source,
            command,
            parameters,
        }
    }
}

fn parse_tags(tags_str: &str) -> HashMap<String, serde_json::Value> {
    let mut tags = HashMap::new();
    for tag in tags_str.split(';') {
        let mut parts = tag.splitn(2, '=');
        let key = parts.next().unwrap();
        let value = parts.next().unwrap_or("").to_string();

        if key == "badges" || key == "emotes" {
            let map_value = parse_special_tag(key, &value);
            tags.insert(key.to_string(), map_value);
        } else {
            tags.insert(key.to_string(), json!(value));
        }
    }
    tags
}

fn parse_special_tag(key: &str, value: &str) -> serde_json::Value {
    let mut map = HashMap::new();
    match key {
        "badges" => {
            for badge in value.split(',') {
                if let Some((badge_key, badge_value)) = badge.split_once('/') {
                    map.insert(badge_key.to_string(), json!(badge_value));
                }
            }
        }
        "emotes" => {
            for emote in value.split('/') {
                let mut parts = emote.split(':');
                let emote_id = parts.next().expect("Emote ID not found in emote string");
                if let Some(positions) = parts.next() {
                    let pos_list: Vec<EmotePosition> = positions
                        .split(',')
                        .map(|pos| {
                            let mut pos_parts = pos.split('-');
                            EmotePosition {
                                start_position: pos_parts
                                    .next()
                                    .expect("Start position not found in emote string")
                                    .to_string(),
                                end_position: pos_parts
                                    .next()
                                    .expect("End position not found in emote string")
                                    .to_string(),
                            }
                        })
                        .collect();

                    map.insert(emote_id.to_string(), json!(pos_list));
                } else {
                    // If there are no positions, insert an empty list
                    map.insert(emote_id.to_string(), json!([]));
                }
            }
        }
        _ => {}
    }
    json!(map)
}

fn parse_source(raw_source_component: &str) -> Source {
    let source_parts: Vec<&str> = raw_source_component.split('!').collect();
    if source_parts.len() == 2 {
        Source {
            nick: Some(source_parts[0].to_string()),
            host: source_parts[1].to_string(),
        }
    } else {
        Source {
            nick: None,
            host: source_parts[0].to_string(),
        }
    }
}

fn parse_command(raw_command_component: &str) -> Option<Command> {
    let command_parts: Vec<&str> = raw_command_component.split_whitespace().collect();
    match command_parts[0] {
        | "JOIN"
        | "PART"
        | "NOTICE"
        | "CLEARCHAT"
        | "HOSTTARGET"
        | "PRIVMSG"
        | "USERSTATE"
        | "ROOMSTATE" =>
            Some(Command {
                command: command_parts[0].to_string(),
                channel: command_parts.get(1).map(|s| s.to_string()),
                is_cap_request_enabled: None,
                bot_command: None,
            }),
        "PING" | "GLOBALUSERSTATE" | "RECONNECT" =>
            Some(Command {
                command: command_parts[0].to_string(),
                channel: None,
                is_cap_request_enabled: None,
                bot_command: None,
            }),
        "CAP" =>
            Some(Command {
                command: command_parts[0].to_string(),
                channel: None,
                is_cap_request_enabled: Some(command_parts.get(2).map_or(false, |s| *s == "ACK")),
                bot_command: None,
            }),
        "001" =>
            Some(Command {
                command: command_parts[0].to_string(),
                channel: command_parts.get(1).map(|s| s.to_string()),
                is_cap_request_enabled: None,
                bot_command: None,
            }),
        "421" | "002" | "003" | "004" | "353" | "366" | "372" | "375" | "376" => None,
        _ => None,
    }
}

fn parse_parameters(params: &str) -> (String, Option<String>) {
    let command_parts: Vec<&str> = params[1..].split_whitespace().collect();
    let bot_command = command_parts[0].to_string();
    let bot_command_params = if command_parts.len() > 1 {
        Some(command_parts[1..].join(" "))
    } else {
        None
    };
    (bot_command, bot_command_params)
}

pub struct PrivateMessageResponse {
    channel: String,
    message: String,
}

impl PrivateMessageResponse {
    pub fn from(channel: &str, message: &str) -> Self {
        Self {
            channel: channel.to_string(),
            message: message.to_string(),
        }
    }
}

impl Display for PrivateMessageResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PRIVMSG {} :{}", self.channel, self.message)
    }
}
