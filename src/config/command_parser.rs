use std::{ collections::HashSet, fs::File };
use std::io::BufReader;
use regex::Regex;
use serde::{ Deserialize, Serialize };
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub name: String,
    pub response: String,
    pub cooldown_in_s: String,
    pub cooldown_scope: String,
}

#[derive(Clone)]
pub struct Commands(Vec<Command>);

impl Commands {
    pub fn new(file_path: &str) -> Self {
        let commands = read_commands_from_file(file_path);

        for command in commands.clone() {
            if let Err(error) = validate_command_placeholders(&command) {
                panic!("{}", error);
            };
        }

        info!("Validated and parsed commands");
        Self(commands)
    }
    pub fn get(&self) -> &Vec<Command> {
        &self.0
    }
}

fn read_commands_from_file(file_path: &str) -> Vec<Command> {
    let Ok(file) = File::open(file_path) else {
        panic!("Failed to open commands.json file");
    };
    let reader = BufReader::new(file);
    let Ok(commands) = serde_json::from_reader(reader) else {
        panic!("Failed to parse commands.json");
    };

    commands
}

fn validate_command_placeholders(command: &Command) -> Result<(), String> {
    let re = Regex::new(r"\{(\w+)\}").unwrap();

    let name_placeholders: HashSet<_> = re
        .captures_iter(&command.name)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str()))
        .collect();

    let response_placeholders: HashSet<_> = re
        .captures_iter(&command.response)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str()))
        .filter(|&placeholder| placeholder != "sender") // Ignore {sender}
        .collect();

    if name_placeholders != response_placeholders {
        return Err(
            format!(
                "Placeholder mismatch in command '{}': {:?} (name) != {:?} (response)",
                command.name,
                name_placeholders,
                response_placeholders
            )
        );
    }

    Ok(())
}
