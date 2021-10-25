use clap::{App, Arg};
use dirs::document_dir;
use sled::{self, Db};
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::PathBuf,
    str::FromStr,
};
use warp::Filter;

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

    let db_result = setup_database(directory.clone());

    if db_result.is_ok() {
        // GET /hello/warp => 200 OK with body "Hello, warp!"
        let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));

        warp::serve(hello).run(host_addr).await;
    } else {
        println!("Failed to set up database");
    }
}
