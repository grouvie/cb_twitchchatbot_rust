use tokio::{ io::{ split, AsyncBufReadExt, AsyncWriteExt, BufReader }, net::TcpStream, sync::mpsc };
use tokio_native_tls::native_tls::TlsConnector;
use tokio_native_tls::TlsConnector as TokioTlsConnector;
use tracing::{ info, error };

pub struct TcpHandler {
    nickname: String,
    oauth_token: String,
    channel: String,
    from_tcp_sender: mpsc::UnboundedSender<String>,
}

impl TcpHandler {
    pub fn new(
        nickname: &str,
        oauth_token: &str,
        channel: &str,
        from_tcp_sender: mpsc::UnboundedSender<String>
    ) -> Self {
        Self {
            nickname: nickname.to_string(),
            oauth_token: oauth_token.to_string(),
            channel: channel.to_string(),
            from_tcp_sender,
        }
    }

    pub async fn run(&mut self, mut from_bot_receiver: mpsc::UnboundedReceiver<String>) {
        let server_address = "irc.chat.twitch.tv";
        let server_port = 6697;

        // Connect to the server over TCP
        let tcp_stream = TcpStream::connect((server_address, server_port)).await.expect(
            "Connecting to Twitch failed"
        );

        // Set up TLS
        let native_tls_connector = TlsConnector::new().expect("Creating TlsConnector failed");
        let tls_connector = TokioTlsConnector::from(native_tls_connector);
        let tls_stream = tls_connector
            .connect(server_address, tcp_stream).await
            .expect("TLS connection failed");

        let (read_half, mut write_half) = split(tls_stream);

        info!("Connected to Twitch IRC server");

        // Clone the sender to be able to move it into a thread
        let from_tcp_sender_into = self.from_tcp_sender.clone();

        // Task to read from TCP and send to internal channel
        tokio::spawn(async move {
            let mut reader = BufReader::new(read_half);
            let mut buffer = String::new();
            info!("Reading from TCP started");

            while reader.read_line(&mut buffer).await.is_ok() {
                let raw_message = buffer.trim_end().to_string();

                buffer.clear();

                if let Err(error) = from_tcp_sender_into.send(raw_message) {
                    error!("Sending message to chat_bot in tcp_handler failed: {}", error);
                    break;
                }
            }
        });

        // Clone token, nickname and channel to be able to move them into a thread
        let oauth_token_into = self.oauth_token.clone();
        let nickname_into = self.nickname.clone();
        let channel_into = self.channel.clone();

        // Task to read from internal channel and write back to TCP
        tokio::spawn(async move {
            info!("Writing to TCP started");
            write_half
                .write_all(format!("PASS {}\r\n", oauth_token_into).as_bytes()).await
                .unwrap();
            // Authenticate
            write_half.write_all(format!("NICK {}\r\n", nickname_into).as_bytes()).await.unwrap();
            write_half.write_all(format!("JOIN #{}\r\n", channel_into).as_bytes()).await.unwrap();
            write_half.write_all("CAP REQ :twitch.tv/tags\r\n".as_bytes()).await.unwrap();
            write_half.write_all("CAP REQ :twitch.tv/tags\r\n".as_bytes()).await.unwrap();

            while let Some(message) = from_bot_receiver.recv().await {
                write_half.write_all(format!("{}\r\n", message).as_bytes()).await.unwrap();
            }
        });
    }
}
