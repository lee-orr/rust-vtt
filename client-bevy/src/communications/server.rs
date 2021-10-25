use std::{collections::HashMap, net::SocketAddr, sync::{Arc, Mutex, atomic::{AtomicI32, AtomicUsize}}};
use async_compat::Compat;
use crossbeam_channel::{Sender, unbounded, Receiver};
use futures_util::{SinkExt, StreamExt};
use tokio::{net::{TcpListener, TcpStream}};
use bevy::{prelude::*, tasks::IoTaskPool};
use tokio_tungstenite::{accept_async, tungstenite::{Error, Message}};
use super::shared::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ServerState {
    Closed,
    Open
}

#[derive(Debug, Clone)]
pub struct Client {
    pub id: usize,
    pub sender: tokio::sync::mpsc::Sender<String>,
}

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);
type Clients = Arc<Mutex<HashMap<usize, Client>>>;

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_state(ServerState::Closed)
            .add_system_set(
                SystemSet::on_enter(ServerState::Open)
                    .with_system(setup_server.system())  
            )
            .add_system_set(
                SystemSet::on_update(ServerState::Open)
                    .with_system(message_system.system())
            );
    }
}

fn setup_server(mut commands: Commands, communication: Res<CommunicationResource>, task_pool: Res<IoTaskPool>) {
    if (!communication.running) {
        eprintln!("Not running");
        return;
    }
    if let CommunicationState::Server { port } = communication.state {
        println!("Setting up server");

        let addr = format!("0.0.0.0:{}", port);
        let clients: Clients = Arc::new(Mutex::new(HashMap::new()));
        let (client_to_game_sender, client_to_game_receiver) = unbounded::<String>();

        task_pool
            .spawn(Compat::new(tokio_setup(addr.clone(), clients.clone(), client_to_game_sender))).detach();
        commands.insert_resource(clients);
        commands.insert_resource(client_to_game_receiver);
    } else {
        eprintln!("Can't set up server");
    }
}

async fn tokio_setup(address: String, clients: Clients, client_to_game_sender: Sender<String>) {
    let listener = TcpListener::bind(&address).await;
    if let Ok(listener) = listener {
        println!("Listening on {}", &address);
        while let Ok((stream, _)) = listener.accept().await {
            let peer = stream.peer_addr().expect("Should have peer address");
            println!("Peer connected: {}", peer);

            tokio::spawn(accept_connection(peer, stream, clients.clone(), client_to_game_sender.clone()));
        }
    } else if let Err(error) = listener {
        eprintln!("Couldn't listen on the port. {}", &error);
    }
}

async fn accept_connection(
    peer: SocketAddr,
    stream: TcpStream,
    clients: Clients,
    client_to_game_sender: Sender<String>,
) {
    if let Err(e) = handle_connection(peer, stream, clients, client_to_game_sender).await {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
            err => println!("Error processing connection: {}", err),
        }
    }
}

async fn handle_connection(peer: SocketAddr, stream: TcpStream, clients: Clients, client_to_game_sender: Sender<String>) -> Result<(), Error> {
    let ws_stream = accept_async(stream).await.expect("Couldn't accept connection");
    println!("Accepted Connection {}", peer);
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (game_to_client_sender, mut game_to_client_receiver) = tokio::sync::mpsc::channel(100);
    let id = NEXT_USER_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    clients.lock().unwrap().insert(id, Client { id: id, sender: game_to_client_sender });

    loop {
        tokio::select! {
            msg = ws_receiver.next() => {
                match msg {
                    Some (msg) => {
                        let msg = msg?;
                        if msg.is_text() || msg.is_binary() {
                            let msg = msg.to_string();
                            println!("Recieved {} from {}", msg, id);
                            client_to_game_sender.send(msg).expect("couldn't send message")
                        }
                    },
                    None => break,
                }
            },
            game_msg = game_to_client_receiver.recv() => {
                let game_msg = game_msg.unwrap();
                ws_sender.send(Message::Text(game_msg)).await?;
            }
        }
    }

    clients.lock().unwrap().remove(&id);
    println!("Client Disconnected {}", peer);

    Ok(())
}

fn message_system(clients: Res<Clients>, client_to_game_receiver: Res<Receiver<String>>) {
    for (_id, client) in clients.lock().unwrap().iter() {
        if let Ok(_) = client.sender.try_send("Sent a message".to_string()) {

        } else {
            eprint!("Failed to send a message");
        }
    }

    while let Ok(msg) = client_to_game_receiver.try_recv() {
        println!("Got Message {:?}", msg);
    }
}