use std::collections::{HashMap, HashSet};

use crate::channel::channel_process;
use lunatic::{
    channel::{unbounded, Receiver, Sender},
    Process,
};
use serde::{Deserialize, Serialize};

use crate::channel::ChannelMessage;

struct ServerState {
    clients: usize,
    channels: HashMap<String, ChannelState>,
}

struct ChannelState {
    clients: usize,
    channel: Sender<ChannelMessage>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerInfo {
    pub username: String,
    pub clients: usize,
}

#[derive(Serialize, Deserialize)]
pub enum ServerMessage {
    // Client wants to receive server information.
    Joined(Sender<ServerInfo>),
    // Clients notify the server that they left and send all the the channels to drop.
    Left(Vec<String>),
    // Clients notify the server that they want to join a channel.
    // (channel name, a lunatic channel to send back a lunatic channel to send messages to the channel :S)
    Channel(String, Sender<Sender<ChannelMessage>>),
    // Clients notify the server that they want to drop out of a channel.
    DropChannel(String),
    // Clients requests a name change
    // (from, to, confirmation lunatic channel)
    ChangeName(String, String, Sender<bool>),
}

/// The server is the main coordinator between all the clients.
/// It keeps track of connected clients and active channels.
/// It's also in charge of assigning unique usernames to new clients.
pub fn server_process(state_receiver: Receiver<ServerMessage>) {
    let mut state = ServerState {
        clients: 0,
        channels: HashMap::new(),
    };

    let mut username_generator: i64 = 0;
    let mut all_usernames = HashSet::new();

    loop {
        match state_receiver.receive().unwrap() {
            ServerMessage::Joined(client) => {
                // Increase the number of active users
                state.clients += 1;
                // Generate a new username
                username_generator += 1;
                let username = format!("User_{}", username_generator);
                all_usernames.insert(username.clone());
                // Client specific state
                let server_info = ServerInfo {
                    clients: state.clients,
                    username,
                };
                let _ = client.send(server_info);
            }
            ServerMessage::ChangeName(from, to, client) => {
                if all_usernames.contains(&to) {
                    // Notify client that the name is taken
                    let _ = client.send(false);
                } else {
                    all_usernames.remove(&from);
                    all_usernames.insert(to);
                    let _ = client.send(false);
                }
            }
            ServerMessage::Left(channels) => {
                state.clients -= 1;
                channels.iter().for_each(|name| {
                    if let Some(exists) = state.channels.get_mut(name) {
                        exists.clients -= 1;
                    }
                })
            }
            ServerMessage::DropChannel(name) => {
                let drop = if let Some(exists) = state.channels.get_mut(&name) {
                    exists.clients -= 1;
                    exists.clients == 0
                } else {
                    false
                };
                // Remove channel if last client left
                if drop {
                    state.channels.remove(&name);
                };
            }
            ServerMessage::Channel(channel_name, client) => {
                if let Some(exists) = state.channels.get_mut(&channel_name) {
                    // Channel already exists
                    exists.clients += 1;
                    let _ = client.send(exists.channel.clone());
                } else {
                    // Create a new channel process
                    let (channel_sender, channel_receiver) = unbounded();
                    Process::spawn_with((channel_name.clone(), channel_receiver), channel_process)
                        .detach();
                    let _ = client.send(channel_sender.clone());
                    let channel_state = ChannelState {
                        clients: 1,
                        channel: channel_sender,
                    };
                    state.channels.insert(channel_name, channel_state);
                }
            }
        }
    }
}
