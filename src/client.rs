use crate::{
    channel::ChannelMessage,
    ui::{Tab, TabType, Ui, UiTabs},
};
use crate::{server::ServerMessage, telnet::Telnet};
use crate::{
    telnet::TelnetMessage::{self, *},
    ui::telnet_backend,
};
use askama::Template;
use chrono::{DateTime, Local};
use lunatic::{
    channel::{bounded, unbounded, Sender},
    net::TcpStream,
    Process,
};
use serde::{Deserialize, Serialize};

#[derive(Template)]
#[template(path = "welcome.txt", escape = "none")]
#[derive(Serialize, Deserialize, Clone)]
struct Welcome {
    username: String,
    clients: usize,
}

#[derive(Template)]
#[template(path = "list.txt", escape = "none")]
struct ChannelList {
    list: Vec<(String, usize)>,
}

#[derive(Template)]
#[template(path = "instructions.txt", escape = "none")]
struct Instructions {}

#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
    Telnet(TelnetMessage),
    ChannelMessage(ChannelMessage),
}

pub fn client_process((central_server, tcp_stream): (Sender<ServerMessage>, TcpStream)) {
    let (state_lookup, state) = bounded(1);
    // Let the state process know that we joined
    central_server
        .send(ServerMessage::Joined(state_lookup))
        .unwrap();
    let server_info = state.receive().unwrap();

    let mut username = server_info.username.clone();

    let (interrupt_sender, interrupt_listener) = unbounded();

    let _tcp_listener = Process::spawn_with(
        (tcp_stream.clone(), interrupt_sender.clone()),
        |(tcp_stream, interrupt_sender)| {
            let mut telnet = Telnet::new(tcp_stream);
            telnet.iac_do_linemode().unwrap();
            telnet.iac_linemode_zero();
            telnet.iac_will_echo().unwrap();
            telnet.iac_do_naws().unwrap();

            loop {
                match telnet.next() {
                    Ok(command) => interrupt_sender
                        .send(ClientMessage::Telnet(command))
                        .unwrap(),
                    Err(_) => break,
                };
            }
        },
    );

    let window_size = telnet_backend::WindowSize::new();

    let welcome = Welcome {
        username: server_info.username,
        clients: server_info.clients,
    };
    let tab = Tab::new(
        "Welcome".to_string(),
        None,
        TabType::Info(welcome.render().unwrap()),
    );
    let mut tabs = UiTabs::new(tab);

    let mut ui = Ui::new(tcp_stream, window_size.clone(), tabs.clone());
    loop {
        match interrupt_listener.receive().unwrap() {
            // Handle commands coming from tcp
            ClientMessage::Telnet(command) => match command {
                CtrlC => {
                    break;
                }
                Tab => {
                    tabs.next();
                    ui.render();
                }
                Backspace => {
                    tabs.input_del_char();
                    ui.render();
                }
                Char(ch) => {
                    tabs.input_add_char(ch.into());
                    ui.render();
                }
                Enter => {
                    let input = tabs.clear();
                    let input = input.trim();
                    if input.starts_with("/") {
                        // Command
                        let mut split = input.split(" ");
                        match split.next().unwrap() {
                            "/help" => {
                                let instructions = Instructions {};
                                let tab = Tab::new(
                                    "Help".to_string(),
                                    None,
                                    TabType::Info(instructions.render().unwrap()),
                                );
                                tabs.add(tab);
                                ui.render();
                            }
                            "/nick" => {
                                if let Some(nick) = split.next() {
                                    let (sender, receiver) = bounded(1);
                                    central_server
                                        .send(ServerMessage::ChangeName(
                                            username.clone(),
                                            nick.to_string(),
                                            sender,
                                        ))
                                        .unwrap();
                                    if receiver.receive().unwrap() {
                                        username = nick.to_string();
                                    } else {
                                    }
                                };
                                ui.render();
                            }
                            "/list" => {
                                let (sender, receiver) = bounded(1);
                                central_server.send(ServerMessage::List(sender)).unwrap();
                                let list = receiver.receive().unwrap();
                                let list = ChannelList { list };
                                let tab = Tab::new(
                                    "Channels".to_string(),
                                    None,
                                    TabType::Info(list.render().unwrap()),
                                );
                                tabs.add(tab);
                                ui.render();
                            }
                            "/drop" => {
                                let current_channel = tabs.get_selected().get_name();
                                central_server
                                    .send(ServerMessage::DropChannel(current_channel))
                                    .unwrap();
                                tabs.drop();
                                ui.render();
                            }
                            "/join" => {
                                let channel = split.next().unwrap();
                                if channel.starts_with("#") {
                                    // Request channel from server
                                    let (channel_lookup, channel_rcv) = bounded(1);
                                    central_server
                                        .send(ServerMessage::Channel(
                                            channel.into(),
                                            channel_lookup,
                                        ))
                                        .unwrap();
                                    let channel_notify = channel_rcv.receive().unwrap();
                                    // Subscribe to channel
                                    let (id_sender, id_receiver) = bounded(1);
                                    channel_notify
                                        .send(ChannelMessage::Subscribe(
                                            username.clone(),
                                            interrupt_sender.clone(),
                                            id_sender,
                                        ))
                                        .unwrap();
                                    // Wait on channel to assign an id to client
                                    let id = id_receiver.receive().unwrap();
                                    // Create new tab bound to channel
                                    let tab = Tab::new(
                                        channel.to_string(),
                                        Some((id, channel_notify)),
                                        TabType::Channel(Vec::new()),
                                    );
                                    tabs.add(tab);
                                    ui.render();
                                } else {
                                    // Incorrect channel name
                                }

                                ui.render();
                            }
                            "/exit" => break,
                            _ => {}
                        }
                    } else {
                        // Send to channel
                        if input.len() > 0 {
                            tabs.get_selected()
                                .message(username.clone(), input.to_string());
                        }
                    }
                    ui.render();
                }
                Naws(width, height) => {
                    window_size.set(width, height);
                    ui.render();
                }
                _ => {}
            },

            // Handle messages coming from channels
            ClientMessage::ChannelMessage(message) => match message {
                ChannelMessage::Message(channel, name, message) => {
                    let now: DateTime<Local> = Local::now();
                    let timestamp = format!("[{}] ", now.format("%H:%M UTC"));
                    tabs.add_message(channel, timestamp, name, message);
                    ui.render();
                }
                _ => {}
            },
        }
    }

    // Let the state process know that we left
    central_server
        .send(ServerMessage::Left(username.clone(), tabs.names()))
        .unwrap();
}
