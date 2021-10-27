use crossbeam_channel::unbounded;
use crossbeam_channel::{Receiver, Sender};
use serde::{de::DeserializeOwned, Serialize};

#[cfg(any(feature = "native", feature = "web"))]
use futures_util::{SinkExt, StreamExt};

#[cfg(feature = "native")]
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message},
};

use url::Url;
#[cfg(feature = "web")]
use ws_stream_wasm::*;

#[derive(Debug, Clone)]
pub enum ClientControl {
    Disconnect,
}

pub enum ClientError {
    Generic(String),
    InvalidAddress,
    FailedToConnect,
}

#[derive(Debug)]
pub struct Client<T>
where
    T: Clone + Send + Serialize + DeserializeOwned + 'static,
{
    pub url: Url,
    pub sender: tokio::sync::mpsc::Sender<T>,
    pub receiver: Receiver<(usize, T)>,
    pub control_sender: tokio::sync::mpsc::Sender<ClientControl>,
    sender_endpoint: tokio::sync::mpsc::Receiver<T>,
    control_receiver: tokio::sync::mpsc::Receiver<ClientControl>,
    receiver_endpoint: Sender<(usize, T)>,
}

impl<T> Client<T>
where
    T: Clone + Send + Serialize + DeserializeOwned + 'static,
{
    pub fn new(url: String) -> Result<Self, ClientError> {
        let url = Url::parse(&url);
        if url.is_err() {
            return Err(ClientError::InvalidAddress);
        }
        let url = url.unwrap();
        let (client_to_server_sender, client_to_server_receiver) = unbounded::<(usize, T)>();
        let (server_to_client_sender, server_to_client_receiver) =
            tokio::sync::mpsc::channel::<T>(100);
        let (control_sender, control_receiver) = tokio::sync::mpsc::channel::<ClientControl>(100);
        Ok(Client {
            url,
            sender: server_to_client_sender,
            sender_endpoint: server_to_client_receiver,
            control_receiver,
            control_sender,
            receiver: client_to_server_receiver,
            receiver_endpoint: client_to_server_sender,
        })
    }

    #[cfg(feature = "native")]
    pub async fn start(self) -> Result<(), ClientError> {
        let stream = connect_async(&self.url).await;
        match stream {
            Ok((stream, _)) => {
                println!("Successfully Connected to {}", &self.url);
                let (mut write, mut read) = stream.split();
                let mut control_receiver = self.control_receiver;
                let mut sender_endpoint = self.sender_endpoint;
                let receiver_endpoint = self.receiver_endpoint;
                loop {
                    tokio::select! {
                        msg = read.next() => {
                            match msg {
                                Some(msg) => {
                                    if msg.is_err() {
                                        continue;
                                    }
                                    let msg = msg.unwrap();
                                    match msg {
                                        Message::Text(msg) => {
                                            if let Ok(value) = serde_json::from_str(&msg) {
                                                if receiver_endpoint.send(value).is_err() {
                                                    eprintln!("Failed to send message to game");
                                                }
                                            }
                                        },
                                        _ => {
                                            eprintln!("Couldn't parse - message isn't text");
                                        }
                                    }
                                },
                                None => break,
                            }
                        },
                        send_msg = sender_endpoint.recv() => {
                            if let Some(msg) = send_msg {
                                let msg = serde_json::to_string(&msg);
                                if msg.is_err() {
                                    continue;
                                }
                                let msg = msg.unwrap();
                                println!("Sending message {}", msg);
                                let result = write.send(Message::Text(msg)).await;
                                if result.is_err() {
                                    eprintln!("Failed to send message");
                                }
                            }
                        },
                        _ = control_receiver.recv() => {
                            break;
                        },
                    }
                }
                Ok(())
            }
            Err(_) => {
                eprintln!("Failed to connect to {}", &self.url);
                Err(ClientError::FailedToConnect)
            }
        }
    }

    #[cfg(feature = "web")]
    pub async fn start(mut self) -> Result<(), ClientError> {
        let stream = WsMeta::connect(&self.url, None).await;
        match stream {
            Ok((_, stream)) => {
                println!("Successfully Connected to {}", &self.url);
                let (mut write, mut read) = stream.split();
                let mut control_receiver = self.control_receiver;
                let mut sender_endpoint = self.sender_endpoint;
                let mut receiver_endpoint = self.receiver_endpoint;
                loop {
                    tokio::select! {
                        msg = read.next() => {
                            match msg {
                                Some(msg) => {
                                    match msg {
                                        WsMessage::Text(msg) => {
                                            if let Ok(value) = serde_json::from_str(&msg) {
                                                if receiver_endpoint.send(value).is_err() {
                                                    eprintln!("Failed to send message to game");
                                                }
                                            }
                                        },
                                        _ => {
                                            eprintln!("Couldn't parse - message isn't text");
                                        }
                                    }
                                },
                                None => break,
                            }
                        },
                        send_msg = sender_endpoint.recv() => {
                            if let Some(msg) = send_msg {
                                let msg = serde_json::to_string(&msg);
                                if msg.is_err() {
                                    continue;
                                }
                                let msg = msg.unwrap();
                                println!("Sending message {}", msg);
                                let result = write.send(WsMessage::Text(msg)).await;
                                if result.is_err() {
                                    eprintln!("Failed to send message");
                                }
                            }
                        },
                        _ = control_receiver.recv() => {
                            break;
                        },
                    }
                }
                Ok(())
            }
            Err(_) => {
                eprintln!("Failed to connect to {}", &self.url);
                Err(ClientError::FailedToConnect)
            }
        }
    }
}
