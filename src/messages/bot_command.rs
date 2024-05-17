use std::{ collections::HashMap, sync::{ Arc, Mutex }, time::{ SystemTime, UNIX_EPOCH } };

use regex::Regex;
use serde::{ Deserialize, Serialize };
use tracing::{ error, info, warn };

use crate::{ config::command_parser::Commands, messages::private_message::PrivateMessageResponse };
use crate::config::command_parser::Command;
#[derive(Serialize, Deserialize, Debug)]
pub struct BotCommand {
    pub command: String,
    pub command_params: Option<String>,
}

impl BotCommand {
    pub fn parse(
        &self,
        display_name: &str,
        channel: &str,
        commands: Commands,
        last_triggers: Arc<Mutex<HashMap<String, HashMap<String, u64>>>>
    ) -> Option<PrivateMessageResponse> {
        let command = match self.find_command(&commands) {
            Some(cmd) => cmd,
            None => {
                warn!("Command {} not found", self.command);
                return None;
            } // Command not found
        };

        let response_message = self.replace_sender(&command.response, display_name);

        if !self.check_cooldown(&command, display_name, &last_triggers) {
            return None; // Command is under cooldown
        }

        let response_message = self.replace_placeholders(
            &response_message,
            &command.name,
            &self.command_params
        );

        info!(
            "Handled: {} - {}",
            self.command,
            self.command_params.clone().unwrap_or("".to_string())
        );

        Some(PrivateMessageResponse::from(channel, &response_message))
    }

    fn find_command(&self, commands: &Commands) -> Option<Command> {
        let command = commands
            .get()
            .iter()
            .find(|command| {
                let cmd_name = command.name.split_whitespace().next().unwrap_or("");
                cmd_name == self.command
            });
        command.cloned()
    }

    fn replace_sender(&self, response: &str, display_name: &str) -> String {
        response.replace("{sender}", display_name)
    }

    fn check_cooldown(
        &self,
        command: &Command,
        display_name: &str,
        last_triggers: &Arc<Mutex<HashMap<String, HashMap<String, u64>>>>
    ) -> bool {
        let username = display_name.to_string();
        let cooldown_scope = &command.cooldown_scope;
        let cooldown = command.cooldown_in_s.parse().unwrap_or(0);
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let mut last_triggers = last_triggers.lock().expect("Failed to lock last_triggers");

        match cooldown_scope.as_str() {
            "user" => {
                let user_cooldown = last_triggers.entry(self.command.clone()).or_default();

                if let Some(last_trigger) = user_cooldown.get(&username) {
                    if current_time - *last_trigger < cooldown {
                        info!(
                            "User: {} is still under cooldown for {} command",
                            display_name,
                            self.command
                        );
                        return false;
                    }
                }

                user_cooldown.insert(username, current_time);
            }
            "global" => {
                if let Some(last_trigger_time) = last_triggers.get(&self.command) {
                    if let Some(last_trigger) = last_trigger_time.get("global") {
                        if current_time - *last_trigger < cooldown {
                            info!("Command {} is still under global cooldown", self.command);
                            return false;
                        }
                    }
                }

                let mut cooldowns = HashMap::new();
                cooldowns.insert("global".to_string(), current_time);
                last_triggers.insert(self.command.clone(), cooldowns);
            }
            _ => {
                error!("Invalid cooldown_scope: {}", cooldown_scope);
                return false;
            }
        }

        true
    }

    fn replace_placeholders(
        &self,
        response: &str,
        command_name: &str,
        params: &Option<String>
    ) -> String {
        if let Some(parameters) = params {
            let parts: Vec<&str> = parameters.split_whitespace().collect();
            let re = Regex::new(r"\{(\w+)\}").unwrap();

            let mut response_message = response.to_string();
            let mut placeholders = vec![];

            for cap in re.captures_iter(command_name) {
                if let Some(var_name) = cap.get(1).map(|m| m.as_str()) {
                    if var_name != "sender" {
                        placeholders.push(var_name);
                    }
                }
            }

            for (i, var_name) in placeholders.iter().enumerate() {
                if let Some(replacement) = parts.get(i) {
                    let placeholder = format!("{{{}}}", var_name);
                    response_message = response_message.replace(&placeholder, replacement);
                }
            }

            response_message
        } else {
            response.to_string()
        }
    }
}
