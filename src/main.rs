mod channel;
mod client;
mod coordinator;
mod telnet;
mod ui;

use clap::{Arg, Command};
use lunatic::{net::TcpListener, AbstractProcess, Mailbox, ProcessConfig};

use crate::{client::ClientProcess, coordinator::CoordinatorSup};

#[lunatic::main]
fn main(_: Mailbox<()>) {
    let matches = Command::new("lunatic.chat")
        .version("0.1")
        .author("Bernard K. <me@kolobara.com>")
        .about("A telnet chat server")
        .arg(Arg::new("PORT").help("Sets the listening port for the server"))
        .get_matches();

    // Create a coordinator supervisor and register the coordinator under the "coordinator" name.
    CoordinatorSup::link()
        .start("coordinator".to_owned())
        .unwrap();

    let port: u16 = *matches.get_one("PORT").unwrap_or(&2323);
    println!("Started server on port {}", port);
    let address = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(address).unwrap();

    // Limit client's memory usage to 5 Mb & allow sub-processes.
    let mut client_conf = ProcessConfig::new().unwrap();
    client_conf.set_max_memory(5_000_000);
    client_conf.set_can_spawn_processes(true);

    while let Ok((stream, _)) = listener.accept() {
        ClientProcess::configure(&client_conf)
            .start(stream)
            .unwrap();
    }
}
