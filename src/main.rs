mod channel;
mod client;
mod coordinator;
mod telnet;
mod ui;

use clap::{App, Arg};
use lunatic::{net::TcpListener, process, Config, Environment, Mailbox};

#[lunatic::main]
fn main(_: Mailbox<()>) {
    let matches = App::new("lunatic.chat")
        .version("0.1")
        .author("Bernard K. <me@kolobara.com>")
        .about("A telnet chat server")
        .arg(Arg::new("PORT").about("Sets the listening port for the server"))
        .get_matches();

    // Create a specific environment for clients and limit their memory use to 5 Mb.
    let mut client_conf = Config::new(5_000_000, None);
    client_conf.allow_namespace("lunatic::");
    client_conf.allow_namespace("wasi_snapshot_preview1::random_get");
    client_conf.allow_namespace("wasi_snapshot_preview1::clock_time_get");
    let mut client_env = Environment::new(client_conf).unwrap();
    let client_module = client_env.add_this_module().unwrap();

    // Create a coordinator and register it inside the environment
    let coordinator = process::spawn(coordinator::coordinator_process).unwrap();
    client_env
        .register("coordinator", "1.0.0", coordinator)
        .unwrap();

    let port = matches.value_of("PORT").unwrap_or("2323");
    println!("Started server on port {}", port);
    let address = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(address).unwrap();
    while let Ok((stream, _)) = listener.accept() {
        client_module
            .spawn_with(stream, client::client_process)
            .unwrap();
    }
}
