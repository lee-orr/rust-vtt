use crossbeam_channel::{unbounded, Receiver, Sender};
use futures_util::{SinkExt, StreamExt};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::HashMap,
    net::{AddrParseError, SocketAddr},
    sync::{atomic::AtomicUsize, Arc, Mutex},
};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Error, Message},
};

pub type Clients<T> = Arc<Mutex<HashMap<usize, Client<T>>>>;

#[derive(Debug, Clone)]
pub struct Client<T>
where
    T: Clone + Send + Serialize + DeserializeOwned,
{
    pub id: usize,
    pub sender: tokio::sync::mpsc::Sender<(usize, T)>,
}

#[derive(Debug, Clone)]
pub enum ServerControl {
    CloseServer,
}

#[derive(Debug)]
pub struct Server<T>
where
    T: Clone + Send + Serialize + DeserializeOwned,
{
    pub clients: Clients<T>,
    pub address: SocketAddr,
    pub reciever: Receiver<(usize, T)>,
    sender: Sender<(usize, T)>,
    pub control_sender: tokio::sync::mpsc::Sender<ServerControl>,
    control_reciever: tokio::sync::mpsc::Receiver<ServerControl>,
}

#[derive(Debug)]
pub enum ServerError {
    Generic(String),
    InvalidAddress,
    BindingError,
}

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

impl<T: Clone + Send + Serialize + DeserializeOwned + 'static> Server<T> {
    pub fn new(address: String) -> Result<Self, ServerError> {
        let address: Result<SocketAddr, AddrParseError> = address.parse();
        match address {
            Ok(address) => {
                let clients: Clients<T> = Arc::new(Mutex::new(HashMap::new()));
                let (client_to_game_sender, client_to_game_receiver) = unbounded::<(usize, T)>();
                let (control_sender, control_reciever) =
                    tokio::sync::mpsc::channel::<ServerControl>(1);
                Ok(Server::<T> {
                    clients,
                    address,
                    sender: client_to_game_sender,
                    reciever: client_to_game_receiver,
                    control_reciever,
                    control_sender,
                })
            }
            Err(_) => Err(ServerError::InvalidAddress),
        }
    }

    pub async fn start(mut self) -> Result<(), ServerError> {
        let listener = TcpListener::bind(&self.address).await;
        let control_reciever = &mut self.control_reciever;
        match listener {
            Ok(listener) => {
                println!("Listening on socket {}", &self.address);
                loop {
                    tokio::select! {
                        stream = listener.accept() => {
                            match stream {
                                Ok((stream, _)) => {
                                    let peer = stream.peer_addr();
                                    if let Ok(peer) = peer {
                                        tokio::spawn(accept_connection(
                                            peer,
                                            stream,
                                            self.clients.clone(),
                                            self.sender.clone(),
                                        ));
                                    } else {
                                        eprintln!("Error with server - couldn't connect to peer");
                                        break;
                                    }
                                },
                                Err(_) => {
                                    eprintln!("Server closed unexpectedly");
                                    break;
                                }
                            }
                        },
                        _ = control_reciever.recv() => {
                            println!("Closing server on socket {}", &self.address);
                            break;
                        },
                    }
                }
                println!("Socket should be closed");
            }
            Err(_) => {
                eprintln!("Couldn't listen on socket {}", &self.address);
                return Err(ServerError::BindingError);
            }
        }
        println!("Should really be closed now");
        Ok(())
    }
}

async fn accept_connection<T>(
    peer: SocketAddr,
    stream: TcpStream,
    clients: Clients<T>,
    client_to_game_sender: Sender<(usize, T)>,
) where
    T: Clone + Send + Serialize + DeserializeOwned,
{
    if let Err(e) = handle_connection(peer, stream, clients, client_to_game_sender).await {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
            err => println!("Error processing connection: {}", err),
        }
    }
}

async fn handle_connection<T>(
    peer: SocketAddr,
    stream: TcpStream,
    clients: Clients<T>,
    client_to_game_sender: Sender<(usize, T)>,
) -> Result<(), Error>
where
    T: Clone + Send + Serialize + DeserializeOwned,
{
    let ws_stream = accept_async(stream)
        .await
        .expect("Couldn't accept connection");
    println!("Accepted Connection {}", peer);
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (game_to_client_sender, mut game_to_client_receiver) =
        tokio::sync::mpsc::channel::<(usize, T)>(100);
    let id = NEXT_USER_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    {
        let lock = clients.lock();
        lock.unwrap().insert(
            id,
            Client {
                id,
                sender: game_to_client_sender,
            },
        );
    }

    loop {
        tokio::select! {
            msg = ws_receiver.next() => {
                println!("Maybe got something from {}", id);
                match msg {
                    Some (msg) => {
                        if msg.is_err() {
                            continue;
                        }
                        let msg = msg.unwrap();
                        match msg {
                            Message::Text(msg) => {
                                let str = msg.clone();
                                println!("MSG FROM {}: {}", id, &str);
                                if let Ok(value) = serde_json::from_str(&str) {
                                    if client_to_game_sender.send((id, value)).is_err() {
                                        eprintln!("Failed to send message to game");
                                    }
                                }
                            },
                            _ => {
                                println!("Got unidentified message from {}", id);
                            }
                        }
                    },
                    None => {
                        println!("Stream ended for {}", id);
                        break;
                    },
                }
            },
            game_msg = game_to_client_receiver.recv() => {
                if game_msg.is_none() {
                    continue;
                }
                let game_msg = game_msg.unwrap();
                let game_msg = serde_json::to_string(&game_msg);
                if game_msg.is_err() {
                    continue;
                }
                let game_msg = game_msg.unwrap();
                println!("Sending message {} to {}", game_msg, id);
                let result = ws_sender.send(Message::Text(game_msg)).await;
                if result.is_err() {
                    break;
                }
            }
        }
    }

    {
        let lock = clients.lock();
        lock.unwrap().remove(&id);
    }
    println!("Client Disconnected {}", peer);

    Ok(())
}
