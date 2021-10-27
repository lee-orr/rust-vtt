use bevy::{prelude::*, tasks::IoTaskPool};
use crossbeam_channel::{Receiver, Sender};

#[cfg(any(feature = "native", feature = "web"))]
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;

#[cfg(feature = "native")]
use async_compat::Compat;
#[cfg(feature = "native")]
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Error, Message},
};

use client_lib::Client;

#[cfg(feature = "web")]
use ws_stream_wasm::*;

use super::shared::*;

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set(
            SystemSet::on_enter(ClientState::Open).with_system(setup_client.system()),
        )
        .add_system_set(
            SystemSet::on_update(ClientState::Open).with_system(message_system.system()),
        );
    }
}

fn setup_client(
    mut commands: Commands,
    communication: Res<CommunicationResource>,
    task_pool: Res<IoTaskPool>,
) {
    if !communication.running {
        eprintln!("Not running");
        return;
    }
    if let CommunicationState::Client { url } = &communication.state {
        println!("Setting up client");
        let client = Client::<String>::new(url.clone());

        if let Ok(client) = client {
            commands.insert_resource(client.receiver.clone());
            commands.insert_resource(client.sender.clone());
            commands.insert_resource(client.control_sender.clone());

            #[cfg(feature = "native")]
            task_pool.spawn(Compat::new(client.start())).detach();
            #[cfg(feature = "web")]
            task_pool.spawn(client.start()).detach();
        } else {
            eprintln!("Error setting up client");
        }
    } else {
        eprintln!("Can't set up client");
    }
}

#[cfg(feature = "native")]
async fn tokio_setup(
    url: String,
    client_to_game_sender: Sender<String>,
    mut game_to_client_receiver: mpsc::Receiver<String>,
) -> Result<(), Error> {
    let url = url::Url::parse(&url).unwrap();
    let (stream, _) = connect_async(&url).await.expect("Failed to connect");
    println!("Successfully Connected to {}", &url);
    let (mut write, mut read) = stream.split();
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

fn message_system(
    client_sender: Res<tokio::sync::mpsc::Sender<String>>,
    client_receiver: Res<Receiver<(usize, String)>>,
    mut send_message_reader: EventReader<SendMessageEvent>,
    mut received_messages: ResMut<ReceivedMessages>,
) {
    for msg in send_message_reader.iter() {
        if client_sender.try_send(msg.value.clone()).is_ok() {
        } else {
            eprint!("Failed to send a message");
        }
    }

    while let Ok(msg) = client_receiver.try_recv() {
        println!("Got Message {:?}", msg);
        received_messages.messages.push(msg);
    }
}
