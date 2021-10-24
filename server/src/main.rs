use clap::{App, Arg};
use dirs::document_dir;
use sled::{self, Db};
use tokio::sync::{mpsc, RwLock};
use std::{collections::HashMap, net::{Ipv4Addr, SocketAddr, SocketAddrV4}, path::PathBuf, str::FromStr, sync::{Arc, atomic::{AtomicUsize, Ordering}}};
use warp::{Filter, ws::{Message, WebSocket}};
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use tokio_stream::wrappers::UnboundedReceiverStream;

fn parse_arguments() -> (SocketAddr, PathBuf) {
    let matches = App::new("VTT Server")
        .version("0.1")
        .arg(
            Arg::with_name("host")
                .short("h")
                .long("host")
                .value_name("ADDRESS")
                .help("Sets the bound host")
                .takes_value(true)
                .default_value("0.0.0.0"),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .help("Sets the bound port")
                .takes_value(true)
                .default_value("3030"),
        )
        .arg(
            Arg::with_name("directory")
                .short("d")
                .long("dir")
                .value_name("DIRECTORY")
                .help("Sets the source directory")
                .takes_value(true),
        )
        .get_matches();
    let host = matches.value_of("host").unwrap_or("0.0.0.0");
    let port = matches.value_of("port").unwrap_or("3030");
    let current_directory = String::from(
        document_dir()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default(),
    );
    let directory = matches
        .value_of("directory")
        .unwrap_or_else(|| current_directory.as_str());

    println!("Host: {}", host);
    println!("Port: {}", port);
    println!("Directory: {}", directory);

    let host_addr = SocketAddr::from_str(format!("{}:{}", host, port).as_str())
        .unwrap_or_else(|_| SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 3030)));

    (host_addr, PathBuf::from_str(directory).unwrap_or_default())
}

fn setup_database(directory: &PathBuf) -> Result<Db, sled::Error> {
    let mut file = directory.clone();
    file.push("vtt_db");
    sled::open(file.as_os_str())
}

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);
type Clients = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Message>>>>;

#[tokio::main]
async fn main() {
    println!("Running VTT Server");
    let (host_addr, directory) = parse_arguments();

    let db_result = setup_database(&directory);

    if let Ok(_) = db_result {
        // GET /hello/warp => 200 OK with body "Hello, warp!"
        let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));

        let clients = Clients::default();
        let clients = warp::any().map(move || clients.clone());

        let ws = warp::path("ws")
            .and(warp::ws())
            .and(clients)
            .map(|socket: warp::ws::Ws, clients| {
                socket.on_upgrade(move |socket| client_connected(socket, clients))
            });

        warp::serve(hello.or(ws)).run(host_addr).await;
    } else {
        println!("Failed to set up database");
    }
}

async fn client_connected(ws: WebSocket, clients: Clients) {
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);
    println!("Client {} Connected", my_id);
    let (mut user_ws_tx, mut user_ws_rx) = ws.split();
    let (tx, rx) = mpsc::unbounded_channel();

    let mut rx = UnboundedReceiverStream::new(rx);

    tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            user_ws_tx
                .send(message)
                .unwrap_or_else(|error| {
                    eprintln!("Websocket Send Error: {}", error);
                })
                .await;
        }
    });

    clients.write().await.insert(my_id, tx);

    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprint!("Error {}: {}", my_id, e);
                break;
            },
        };
        if let Ok(s) = msg.to_str() {
            let formatted = format!("{}: {}", my_id, s);
            for (&uid, tx) in clients.read().await.iter() {
                 let result = tx.send(Message::text(formatted.clone()));
                 if let Ok(_) = result {} else {
                     eprint!("Sending error to {}", uid);
                 }
            }
        }
    }

    println!("{} Disconnected", my_id);
}