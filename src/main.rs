#![feature(or_patterns)]

mod channel;
mod client;
mod server;
mod telnet;
mod ui;

use lunatic::channel::unbounded;
use lunatic::net::TcpListener;
use lunatic::Process;

use clap::{App, Arg};

fn main() {
    let matches = App::new("lunatic.chat")
        .version("0.1")
        .author("Bernard K. <me@kolobara.com>")
        .about("A telnet chat server")
        .arg(Arg::new("PORT").about("Sets the listening port for the server"))
        .get_matches();

    // This channel is used to allow communication between the main server process and all connected clients.
    let (state_sender, state_receiver) = unbounded();

    Process::spawn_with(state_receiver, server::server_process).detach();

    let port = matches.value_of("PORT").unwrap_or("8080");
    println!("Started server on port {}", port);
    let address = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(address).unwrap();
    while let Ok(tcp_stream) = listener.accept() {
        Process::spawn_with((state_sender.clone(), tcp_stream), client::client_process).detach();
    }
}
