use clap::{App, Arg};
use dirs::document_dir;
use server_lib::Server;
use sled::{self, Db};
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::PathBuf,
    str::FromStr,
};

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

fn setup_database(mut file: PathBuf) -> Result<Db, sled::Error> {
    file.push("vtt_db");
    sled::open(file.as_os_str())
}
#[tokio::main]
async fn main() {
    println!("Running VTT Server");
    let (host_addr, directory) = parse_arguments();

    let db_result = setup_database(directory);

    if db_result.is_ok() {
        let server = Server::<String>::new(host_addr.to_string());
        if server.is_err() {
            eprintln!("Couldn't set up server");
            return;
        }
        let server = server.unwrap();
        let clients = server.clients.clone();
        let receiver = server.reciever.clone();

        tokio::spawn(server.start());
        while let Ok(msg) = receiver.recv() {
            let clients = clients.lock().unwrap();
            for (id, client) in clients.iter() {
                if msg.0 == *id {
                    continue;
                }
                if client.sender.try_send(msg.to_owned()).is_err() {
                    eprint!("Failed to send a message");
                }
            }
        }
    } else {
        println!("Failed to set up database");
    }
}
