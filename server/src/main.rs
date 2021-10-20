use clap::{App, Arg};
use std::{
    env::{current_dir},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    str::FromStr,
};
use warp::Filter;

#[tokio::main]
async fn main() {
    println!("Running VTT Server");
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
    let current_directory = String::from(current_dir().unwrap().to_str().unwrap_or_default());
    let directory = matches
        .value_of("directory")
        .unwrap_or_else(|| current_directory.as_str());

    println!("Host: {}", host);
    println!("Port: {}", port);
    println!("Directory: {}", directory);

    let host_addr = SocketAddr::from_str(format!("{}:{}", host, port).as_str()).unwrap_or_else(|_|
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 3030)),
    );

    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));

    warp::serve(hello).run(host_addr).await;
}
