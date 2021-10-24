use std::net::TcpStream;

use bevy::prelude::*;
use futures_util::{future, pin_mut, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::{WebSocketStream, connect_async, tungstenite::protocol::Message};
use crossbeam_channel::{unbounded, Sender, Receiver};
use async_compat::Compat;

pub fn connect(url: string, client_recieves: Sender<String>, client_sends: Reciever<String>,) {
    let addr = url::Url::parse(url);
    if let Ok(url) = addr {
        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
        let (write, read) = ws_stream.split();
        tokio::spawn(setup_sends(write, client_sends.clone()));
        tokio::spwan(setup_recieves(read, client_recieves.clone()));

    } else {
        eprintln!("Couldn't parse url");
    }
}

async fn setup_sends(write: SplitSink<WebSocketStream<TcpStream>, Message>, client_sends: Receiver<String>) {

}

async fn setup_recieves(read: SplitStream<WebSocketStream<<TcpStream>, Message>>, client_recieves: Sender<String>) {
    read.for_each(|message| async {
        let data = message.to_string();
        client_recieves.send(data);
    }) 
}