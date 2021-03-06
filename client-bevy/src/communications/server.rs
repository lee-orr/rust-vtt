use super::shared::*;
use async_compat::Compat;
use bevy::{prelude::*, tasks::IoTaskPool};
use crossbeam_channel::Receiver;
use server_lib::{Clients, Server, ServerControl};
use tokio::sync::mpsc::Sender;
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_enter(ServerState::Open).with_system(setup_server.system()),
        )
        .add_system_set(
            SystemSet::on_update(ServerState::Open)
                .with_system(message_system.system())
                .with_system(close_server.system()),
        );
    }
}

fn setup_server(
    mut commands: Commands,
    communication: Res<CommunicationResource>,
    task_pool: Res<IoTaskPool>,
) {
    if !communication.running {
        eprintln!("Not running");
        return;
    }
    if let CommunicationState::Server { port } = communication.state {
        println!("Setting up server");
        let server = Server::<String>::new(format!("0.0.0.0:{}", port));

        if let Ok(server) = server {
            commands.insert_resource(server.clients.clone());
            commands.insert_resource(server.reciever.clone());
            commands.insert_resource(server.control_sender.clone());
            task_pool.spawn(Compat::new(server.start())).detach();
        } else {
            eprintln!("Error setting up server");
        }
    } else {
        eprintln!("Can't set up server");
    }
}

fn message_system(
    clients: Res<Clients<String>>,
    client_to_game_receiver: Res<Receiver<(usize, String)>>,
    mut send_message_reader: EventReader<SendMessageEvent>,
    mut received_messages: ResMut<ReceivedMessages>,
) {
    let mut messages: Vec<(usize, String)> = send_message_reader
        .iter()
        .map(|val| (0, val.value.clone()))
        .collect();

    while let Ok((client, msg)) = client_to_game_receiver.try_recv() {
        println!("Got Message {:?}", &msg);
        received_messages.messages.push((client, msg.clone()));
        messages.push((client, msg));
    }

    let mut clients = clients.lock().unwrap();
    let mut failures: Vec<usize> = Vec::new();
    for (id, client) in clients.iter() {
        for msg in messages.iter() {
            if msg.0 == *id {
                continue;
            }
            if client.sender.try_send(msg.to_owned()).is_err() {
                eprint!("Failed to send a message");
                failures.push(*id);
            }
        }
    }
    for id in failures.iter() {
        clients.remove(id);
    }
}

fn close_server(
    control: Option<Res<Sender<ServerControl>>>,
    mut event: EventReader<CloseServerEvent>,
) {
    let control = match control {
        Some(it) => it,
        _ => return,
    };
    if event.iter().next().is_some() {
        eprintln!("Closing Server");
        if control.blocking_send(ServerControl::CloseServer).is_err() {
            eprintln!("Couldn't close server");
        }
    }
}
