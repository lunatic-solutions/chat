#![feature(or_patterns)]

mod telnet;
mod ui;

use askama::Template;
use lunatic::channel::{bounded, unbounded, Receiver, Sender};
use lunatic::net::{TcpListener, TcpStream};
use lunatic::Process;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Write};

use telnet::ClientMessage::*;

#[derive(Template)]
#[template(path = "welcome.txt", escape = "none")]
#[derive(Serialize, Deserialize, Clone)]
// Used to display welcome information
struct ServerInfo {
    username: String,
    clients: usize,
}

struct ServerState {
    clients: usize,
    channels: HashMap<String, Vec<TcpStream>>,
}

#[derive(Serialize, Deserialize)]
enum StateMessage {
    Joined(Sender<ServerInfo>),
    Left,
}

fn main() {
    let (state_sender, state_receiver) = unbounded();
    let state_server =
        Process::spawn_with(state_receiver, |state_receiver: Receiver<StateMessage>| {
            let mut state = ServerState {
                clients: 0,
                channels: HashMap::new(),
            };

            loop {
                let mut username_generator: i64 = 0;

                match state_receiver.receive().unwrap() {
                    StateMessage::Joined(client) => {
                        // Increase the number of active users
                        state.clients += 1;
                        // Generate a new username
                        username_generator += 1;
                        // Client specific state
                        let server_info = ServerInfo {
                            clients: state.clients,
                            username: format!("User_{}", username_generator),
                        };
                        client.send(server_info).unwrap()
                    }
                    StateMessage::Left => state.clients -= 1,
                }
            }
        });
    state_server.detach();

    let listener = TcpListener::bind("0.0.0.0:1337").unwrap();
    while let Ok(tcp_stream) = listener.accept() {
        Process::spawn_with((state_sender.clone(), tcp_stream), client).detach();
    }
}

fn client((state_sender, mut tcp_stream): (Sender<StateMessage>, TcpStream)) {
    let (state_lookup, state) = bounded(1);
    // Let the state process know that we joined
    state_sender
        .send(StateMessage::Joined(state_lookup))
        .unwrap();

    let current_state = state.receive().unwrap();
    // Render welcome message
    // tcp_stream
    //     .write(current_state.render().unwrap().as_bytes())
    //     .unwrap();

    let mut username = current_state.username;

    // write!(tcp_stream, "\u{001B}[2J").unwrap();

    let mut telnet = telnet::Telnet::new(tcp_stream.clone());
    telnet.iac_do_linemode().unwrap();
    telnet.iac_linemode_zero();
    telnet.iac_will_echo().unwrap();
    telnet.iac_do_naws().unwrap();

    let window_size = ui::telnet_backend::WindowSize::new();
    let mut ui = ui::Ui::new(tcp_stream, window_size.clone());

    loop {
        match telnet.next().unwrap() {
            Char(ch) => {
                //
            }
            Naws(width, height) => {
                window_size.set(width, height);
                ui.render();
            }
            _ => {}
        }
        // // Prompt
        // tcp_stream.write("> ".as_bytes()).unwrap();
        // tcp_stream.flush().unwrap();

        // let mut buffer = String::new();
        // match buf_reader.read_line(&mut buffer) {
        //     Ok(size) => {
        //         if size == 0 || buffer.starts_with("/exit") {
        //             break;
        //         } else if buffer.starts_with("/nick ") {
        //             let new_username: Vec<&str> = buffer.split(" ").skip(1).collect();
        //             username = new_username.join(" ");
        //             tcp_stream
        //                 .write(format!("username changed to: {}", username).as_bytes())
        //                 .unwrap();
        //             tcp_stream.flush().unwrap();
        //         }
        //     }
        //     Err(_) => {
        //         tcp_stream
        //             .write("ERROR: **Unsupported character in message or command**".as_bytes())
        //             .unwrap();
        //         tcp_stream.flush().unwrap();
        //     }
        // }
    }

    // Let the state process know that we left
    state_sender.send(StateMessage::Left).unwrap();
}
