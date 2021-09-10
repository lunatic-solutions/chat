use crate::channel::ChannelMessage;
use crate::{coordinator::CoordinatorRequest, telnet::Telnet};
use crate::{
    coordinator::CoordinatorResponse,
    ui::{Tab, TabType, Ui, UiTabs},
};
use crate::{
    telnet::TelnetMessage::{self, *},
    ui::telnet_backend,
};
use askama::Template;
use chrono::{DateTime, Local};
use lunatic::Tag;
use lunatic::{
    lookup,
    net::TcpStream,
    process::{self, Process},
    Mailbox,
};
use serde::{Deserialize, Serialize};

// The template for the welcome screen.
#[derive(Template)]
#[template(path = "welcome.txt", escape = "none")]
#[derive(Serialize, Deserialize, Clone)]
struct Welcome {
    username: String,
    clients: usize,
}

// The template for the list of all channels screen.
#[derive(Template)]
#[template(path = "list.txt", escape = "none")]
struct ChannelList {
    list: Vec<(String, usize)>,
}

// The template for the instructions screen
#[derive(Template)]
#[template(path = "instructions.txt", escape = "none")]
struct Instructions {}

#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
    Telnet(TelnetMessage),
    Channel(Channel),
}

#[derive(Serialize, Deserialize)]
pub enum Channel {
    Message(String, String, String, String),
    LastMessages(Vec<(String, String, String)>),
}

#[derive(Serialize, Deserialize)]
pub struct TelnetContext {
    pub stream: TcpStream,
    pub client: Process<ClientMessage>,
}

pub fn client_process(stream: TcpStream, mailbox: Mailbox<ClientMessage>) {
    // Username of the client
    let mut username;
    // The number of all clients on the server.
    let total_clients;

    // Look up the coordinator or fail if it doesn't exist.
    //
    // Notice that the the `Process<T>` type returned here can't be checked during compile time, as
    // it can be arbitrary. Sending a message of wrong type to the coordinator will fail during
    // runtime only, because the deserialization step will fail.
    let coordinator = lookup("coordinator", "1.0.0").unwrap().unwrap();
    // Let the coordinator know that we joined.
    if let CoordinatorResponse::ServerJoined(client_info) =
        coordinator.request(CoordinatorRequest::JoinServer).unwrap()
    {
        // Update username with an coordinator auto generated one.
        username = client_info.username;
        total_clients = client_info.total_clients;
    } else {
        unreachable!("Received unexpected message");
    }

    // This process is in charge of turning the raw tcp stream into higher level messages that are
    // sent to the client. It's linked to the client and if one of them fails the other will too.
    let this = process::this(&mailbox);
    let (_, mailbox) = process::spawn_link_unwrap_with(
        mailbox,
        (this, stream.clone()),
        |(client, stream), _: Mailbox<()>| {
            let mut telnet = Telnet::new(stream);
            telnet.iac_do_linemode().unwrap();
            telnet.iac_linemode_zero();
            telnet.iac_will_echo().unwrap();
            telnet.iac_do_naws().unwrap();

            loop {
                match telnet.next() {
                    Ok(command) => client.send(ClientMessage::Telnet(command)),
                    Err(err) => panic!("A telnet error ocurred: {:?}", err),
                };
            }
        },
    )
    .unwrap();

    let window_size = telnet_backend::WindowSize::new();

    let welcome = Welcome {
        username: username.clone(),
        clients: total_clients,
    };
    let tab = Tab::new(
        "Welcome".to_string(),
        None,
        TabType::Info(welcome.render().unwrap()),
    );
    let mut tabs = UiTabs::new(tab);

    let mut ui = Ui::new(stream, window_size.clone(), tabs.clone());
    loop {
        let message = mailbox.receive();
        match message.unwrap() {
            // Handle commands coming from Telnet
            ClientMessage::Telnet(command) => match command {
                CtrlC | Error => {
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
                                    let nick = nick.to_string();
                                    if let CoordinatorResponse::NewUsername(new_username) =
                                        coordinator
                                            .request(CoordinatorRequest::ChangeName(nick))
                                            .unwrap()
                                    {
                                        username = new_username;
                                    };
                                };
                                ui.render();
                            }
                            "/list" => {
                                if let CoordinatorResponse::ChannelList(list) = coordinator
                                    .request(CoordinatorRequest::ListChannels)
                                    .unwrap()
                                {
                                    let list = ChannelList { list };
                                    let tab = Tab::new(
                                        "Channels".to_string(),
                                        None,
                                        TabType::Info(list.render().unwrap()),
                                    );
                                    tabs.add(tab);
                                };
                                ui.render();
                            }
                            "/drop" => {
                                let current_channel = tabs.get_selected().get_name();
                                // If the tab is a channel notify coordinator that we are leaving.
                                if current_channel.starts_with("#") {
                                    let result = coordinator
                                        .request(CoordinatorRequest::LeaveChannel(current_channel))
                                        .unwrap();
                                    assert_eq!(result, CoordinatorResponse::ChannelDropped);
                                }
                                tabs.drop();
                                ui.render();
                            }
                            "/join" => {
                                let channel_name = if let Some(channel_name) = split.next() {
                                    channel_name
                                } else {
                                    continue;
                                };
                                if channel_name.starts_with("#") {
                                    let channel_name = channel_name.to_string();
                                    let this = process::this(&mailbox);
                                    if let CoordinatorResponse::ChannelJoined(channel) = coordinator
                                        .request(CoordinatorRequest::JoinChannel(
                                            channel_name.clone(),
                                            this.clone(),
                                        ))
                                        .unwrap()
                                    {
                                        // Get last messages from channel
                                        let tag = Tag::new();
                                        channel.tag_send(tag, ChannelMessage::LastMessages(this));
                                        if let ClientMessage::Channel(Channel::LastMessages(
                                            last_messages,
                                        )) = mailbox.tag_receive(tag).unwrap()
                                        {
                                            // Create new tab bound to channel
                                            let tab = Tab::new(
                                                channel_name,
                                                Some(channel),
                                                TabType::Channel(last_messages),
                                            );
                                            tabs.add(tab);
                                        };
                                    };
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
                        if input.len() > 0 && input.len() < 300 {
                            let now: DateTime<Local> = Local::now();
                            let timestamp = format!("[{}] ", now.format("%H:%M UTC"));
                            tabs.get_selected().message(
                                timestamp.clone(),
                                username.clone(),
                                input.to_string(),
                            );
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
            ClientMessage::Channel(message) => match message {
                Channel::Message(channel, timestamp, name, message) => {
                    tabs.add_message(channel, timestamp, name, message);
                    ui.render();
                }
                _ => {}
            },
        }
    }

    // Let the state process know that we left
    coordinator
        .request(CoordinatorRequest::LeaveServer)
        .unwrap();
}
