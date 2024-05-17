use std::{ collections::HashMap, sync::{ Arc, Mutex } };

use tokio::sync::mpsc;
use tracing::{ error, info };

use crate::{
    config::command_parser::Commands,
    messages::{ bot_command::BotCommand, private_message::PrivateMessageRequest },
    tcp_handler::TcpHandler,
};

pub struct ChatBot {
    sender: mpsc::UnboundedSender<String>,
    receiver: mpsc::UnboundedReceiver<String>,
    commands: Commands,
    last_triggers: Arc<Mutex<HashMap<String, HashMap<String, u64>>>>,
}

impl ChatBot {
    pub fn new(nickname: String, oauth_token: String, channel: String, file_path: String) -> Self {
        let (from_bot_sender, from_bot_receiver) = mpsc::unbounded_channel();

        let (from_tcp_sender, from_tcp_receiver) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            let mut tcp_handler = TcpHandler::new(
                &nickname,
                &oauth_token,
                &channel,
                from_tcp_sender
            );
            tcp_handler.run(from_bot_receiver).await;
        });

        let commands = Commands::new(&file_path);

        let last_triggers: Arc<Mutex<HashMap<String, HashMap<String, u64>>>> = Arc::new(
            Mutex::new(HashMap::new())
        );

        Self {
            sender: from_bot_sender,
            receiver: from_tcp_receiver,
            commands,
            last_triggers,
        }
    }
    fn handle_bot_command(
        &self,
        bot_command: &BotCommand,
        tags: &Option<HashMap<String, serde_json::Value>>,
        channel: &str
    ) -> Option<String> {
        let display_name = tags
            .as_ref()
            .and_then(|tags| tags.get("display-name").and_then(|v| v.as_str()));

        if let Some(display_name) = display_name {
            bot_command
                .parse(display_name, channel, self.commands.clone(), self.last_triggers.clone())
                .map(|response| format!("{}", response))
        } else {
            error!("No display_name in handle_bot_command");
            None
        }
    }

    fn handle_message(&self, private_message_request: &PrivateMessageRequest) -> Option<String> {
        private_message_request.command.as_ref().and_then(|command| {
            match command.command.as_str() {
                "PRIVMSG" => {
                    if let Some(ref bot_command) = command.bot_command {
                        if let Some(ref channel) = command.channel {
                            return self.handle_bot_command(
                                bot_command,
                                &private_message_request.tags,
                                channel.as_str()
                            );
                        }
                    }
                    None
                }
                "PING" => {
                    info!(
                        "{} - {}",
                        command.command,
                        private_message_request.parameters.as_deref().unwrap_or_default()
                    );
                    private_message_request.parameters
                        .as_ref()
                        .map(|parameters| { format!("PONG {}", parameters) })
                }
                _ => {
                    info!(
                        "Unhandled: {} - {}",
                        command.command,
                        private_message_request.parameters.as_deref().unwrap_or_default()
                    );
                    None
                }
            }
        })
    }

    pub async fn run(&mut self) {
        while let Some(raw_message) = self.receiver.recv().await {
            let private_message_request = PrivateMessageRequest::new(&raw_message);
            if let Some(message) = self.handle_message(&private_message_request) {
                if let Err(error) = self.sender.send(message) {
                    error!("Sending to tcp_handler from chat_bot failed {}", error);
                }
            }
        }
    }
}
