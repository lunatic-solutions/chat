use std::process::exit;

use crate::channel::ChannelProcessHandler;
use crate::coordinator::{CoordinatorProcess, CoordinatorProcessHandler};
use crate::telnet::Telnet;
use crate::ui::telnet_backend::WindowSize;
use crate::ui::{Tab, TabType, Ui, UiTabs};
use crate::{
    telnet::TelnetMessage::{self, *},
    ui::telnet_backend,
};
use askama::Template;
use chrono::{DateTime, Local};
use lunatic::process::ProcessRef;
use lunatic::{abstract_process, Process};
use lunatic::{net::TcpStream, Mailbox};
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

/// The client process is spawned for each new telnet connection to the server.
///
/// It receives the `TcpStream` of the connection as argument. Then the client will spawn a
/// sub-process that uses the `TcpStream` to create higher level commands from low-level telnet
/// stream and forward the commands to the client.
///
/// The client will re-render the UI based on messages it receives from the coordinator, channels
/// or telnet sub-process.
pub struct ClientProcess {
    this: ProcessRef<ClientProcess>,
    coordinator: ProcessRef<CoordinatorProcess>,
    username: String,
    tabs: UiTabs,
    ui: Ui,
    window_size: WindowSize,
}

#[abstract_process(visibility = pub)]
impl ClientProcess {
    #[init]
    fn init(this: ProcessRef<Self>, stream: TcpStream) -> Self::State {
        // Look up the coordinator or fail if it doesn't exist.
        let coordinator = ProcessRef::<CoordinatorProcess>::lookup("coordinator").unwrap();
        // Link coordinator to child. The coordinator sets `die_when_link_dies` to `0` and will not fail if child fails.
        coordinator.link();
        // Let the coordinator know that we joined.
        let client_info = coordinator.join_server(this.clone());

        // This process is in charge of turning the raw tcp stream into higher level messages that are
        // sent back to the client. It's linked to the client and if one of them fails the other will too.
        Process::spawn_link(
            (this.clone(), stream.clone()),
            |(client, stream), _: Mailbox<()>| {
                let mut telnet = Telnet::new(stream);
                telnet.iac_do_linemode().unwrap();
                telnet.iac_linemode_zero();
                telnet.iac_will_echo().unwrap();
                telnet.iac_do_naws().unwrap();

                loop {
                    match telnet.next() {
                        Ok(message) => client.process(message),
                        Err(err) => panic!("A telnet error ocurred: {:?}", err),
                    };
                }
            },
        );

        let window_size = telnet_backend::WindowSize::new();
        let welcome = Welcome {
            username: client_info.username.clone(),
            clients: client_info.total_clients,
        };
        let tab = Tab::new(
            "Welcome".to_string(),
            None,
            TabType::Info(welcome.render().unwrap()),
        );
        let tabs = UiTabs::new(tab);
        let ui = Ui::new(stream, window_size.clone(), tabs.clone());

        ClientProcess {
            this,
            coordinator,
            username: client_info.username,
            tabs,
            ui,
            window_size,
        }
    }

    /// Handle data coming in over TCP from telnet.
    #[handle_message]
    fn process(&mut self, command: TelnetMessage) {
        match command {
            CtrlC | Error => {
                self.this.exit();
            }
            Tab => {
                self.tabs.next();
                self.ui.render();
            }
            Backspace => {
                self.tabs.input_del_char();
                self.ui.render();
            }
            Char(ch) => {
                self.tabs.input_add_char(ch.into());
                self.ui.render();
            }
            Enter => {
                let input = self.tabs.clear();
                let input = input.trim();
                if input.starts_with('/') {
                    // Command
                    let mut split = input.split(' ');
                    match split.next().unwrap() {
                        "/help" => {
                            let instructions = Instructions {};
                            let tab = Tab::new(
                                "Help".to_string(),
                                None,
                                TabType::Info(instructions.render().unwrap()),
                            );
                            self.tabs.add_or_switch(tab);
                            self.ui.render();
                        }
                        "/nick" => {
                            if let Some(nick) = split.next() {
                                self.username = self
                                    .coordinator
                                    .change_name(self.this.clone(), nick.to_owned());
                            };
                            self.ui.render();
                        }
                        "/list" => {
                            let list = self.coordinator.list_channels();
                            let list = ChannelList { list };
                            let tab = Tab::new(
                                "Channels".to_string(),
                                None,
                                TabType::Info(list.render().unwrap()),
                            );
                            self.tabs.add_or_switch(tab);
                            self.ui.render();
                        }
                        "/drop" => {
                            let current_channel = self.tabs.get_selected().get_name();
                            // If the tab is a channel notify coordinator that we are leaving.
                            if current_channel.starts_with('#') {
                                self.coordinator
                                    .leave_channel(self.this.clone(), current_channel);
                            }
                            self.tabs.drop();
                            self.ui.render();
                        }
                        "/join" => {
                            let channel_name = if let Some(channel_name) = split.next() {
                                channel_name
                            } else {
                                return;
                            };
                            if channel_name.starts_with('#') {
                                let channel = self
                                    .coordinator
                                    .join_channel(self.this.clone(), channel_name.to_owned());

                                // Get last messages from channel
                                let last_messages = channel.get_last_messages();
                                // Create new tab bound to channel
                                let tab = Tab::new(
                                    channel_name.to_owned(),
                                    Some(channel),
                                    TabType::Channel(last_messages),
                                );
                                self.tabs.add_or_switch(tab);
                            } else {
                                // Incorrect channel name
                            }
                            self.ui.render();
                        }
                        "/exit" => {
                            self.this.exit();
                        }
                        _ => {}
                    }
                } else {
                    // Send to channel
                    if !input.is_empty() && input.len() < 300 {
                        let now: DateTime<Local> = Local::now();
                        let timestamp = format!("[{}] ", now.format("%H:%M UTC"));
                        self.tabs.get_selected().message(
                            timestamp,
                            self.username.clone(),
                            input.to_string(),
                        );
                    }
                }
                self.ui.render();
            }
            Naws(width, height) => {
                self.window_size.set(width, height);
                self.ui.render();
            }
            _ => {}
        }
    }

    /// Handle messages sent by a channel to us.
    #[handle_message]
    fn receive_message(
        &mut self,
        channel: String,
        timestamp: String,
        name: String,
        message: String,
    ) {
        self.tabs.add_message(channel, timestamp, name, message);
        self.ui.render();
    }

    /// Clean up on exit.
    #[handle_message]
    fn exit(&mut self) {
        // Let the coordinator know that we left
        self.coordinator.leave_server(self.this.clone());
        // `exit(1)` is used to kill the linked telnet sub-process, because lunatic doesn't provide a
        // `kill process` API yet.
        exit(1);
    }
}
