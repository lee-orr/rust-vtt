
use bevy::{prelude::*, tasks::IoTaskPool};
use crossbeam_channel::{Sender, unbounded, Receiver};

use tokio::{sync::mpsc};
use futures_util::{SinkExt, StreamExt,};

#[cfg(feature = "native")]
use async_compat::Compat;
#[cfg(feature = "native")]
use tokio_tungstenite::{connect_async, tungstenite::{Error, Message}};

#[cfg(feature = "web")]
use ws_stream_wasm::*;

use super::shared::*;

#[derive(Debug, Clone)]
pub struct Client {
    pub sender: Option<mpsc::Sender<String>>
}

impl Default for Client {
    fn default() -> Self {
        Client { sender: None }
    }
}

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .insert_resource(Client::default())
            .add_system_set(
                SystemSet::on_enter(ClientState::Open)
                    .with_system(setup_client.system())  
            )
            .add_system_set(
                SystemSet::on_update(ClientState::Open)
                    .with_system(message_system.system())
            );
    }
}

fn setup_client(mut commands: Commands, communication: Res<CommunicationResource>, task_pool: Res<IoTaskPool>, mut client: ResMut<Client>) {
    if (!communication.running) {
        eprintln!("Not running");
        return;
    }
    if let CommunicationState::Client { url } = &communication.state {
        println!("Setting up client");
        let (client_to_game_sender, client_to_game_receiver) = unbounded::<String>();
        let (game_to_client_sender, mut game_to_client_receiver) = mpsc::channel(100);

        
        #[cfg(feature = "native")]
        task_pool
            .spawn(Compat::new(tokio_setup(url.clone(), client_to_game_sender, game_to_client_receiver))).detach();
        #[cfg(not(feature = "native"))]
        task_pool.spawn(tokio_setup(url.clone(), client_to_game_sender, game_to_client_receiver)).detach();
        commands.insert_resource(client_to_game_receiver);
        client.sender = Some(game_to_client_sender);
    } else {
        eprintln!("Can't set up client");
    }
}


#[cfg(feature = "native")]
async fn tokio_setup(url: String, client_to_game_sender: Sender<String>, mut game_to_client_receiver: mpsc::Receiver<String>) -> Result<(), Error>{
    let url = url::Url::parse(&url).unwrap();
    let (stream, _) = connect_async(&url).await.expect("Failed to connect");
    println!("Successfully Connected to {}", &url);
    let (mut write, mut read)  = stream.split();
    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some (msg) => {
                        let msg = msg?;
                        if msg.is_text() || msg.is_binary() {
                            let msg = msg.to_string();
                            println!("Recieved {}", msg);
                            client_to_game_sender.send(msg).expect("couldn't send message")
                        }
                    },
                    None => break,
                }
            },
            game_msg = game_to_client_receiver.recv() => {
                let game_msg = game_msg.unwrap();
                write.send(Message::Text(game_msg)).await?;
            }
        }
    }
    
    Ok(())
}


#[cfg(feature = "web")]
async fn tokio_setup(url: String, client_to_game_sender: Sender<String>, mut game_to_client_receiver: mpsc::Receiver<String>) -> Result<(), String>{
    use wasm_bindgen::UnwrapThrowExt;

    let (_ws, stream) = WsMeta::connect(&url, None).await.expect_throw("Connection should work");
    println!("Successfully Connected to {}", &url);

    let (mut write, mut read)  = stream.split();
    
    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some (msg) => {
                        if let WsMessage::Text(msg) = msg {
                            println!("Recieved {}", msg);
                            client_to_game_sender.send(msg).expect("couldn't send message")
                        }
                    },
                    None => break,
                }
            },
            game_msg = game_to_client_receiver.recv() => {
                let game_msg = game_msg.unwrap();
                write.send(WsMessage::Text(game_msg)).await;
            }
        }
    }
   Ok(())
}


fn message_system(client: Res<Client>, client_to_game_receiver: Res<Receiver<String>>) {
    if let Some(sender) = &client.sender {
        if let Ok(_) = sender.try_send("Sent a client message".to_string()) {
    } else {
        eprint!("Failed to send a message");
    }
    }

    while let Ok(msg) = client_to_game_receiver.try_recv() {
        println!("Got Message {:?}", msg);
    }
}