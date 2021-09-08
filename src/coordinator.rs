use std::collections::{HashMap, HashSet};

use crate::{
    channel::{channel_process, ChannelMessage},
    client::ClientMessage,
};

use lunatic::{
    process::{self, Process},
    Mailbox, Message, Request, Tag, TransformMailbox,
};
use serde::{Deserialize, Serialize};

/// All requests a coordinator can get.
#[derive(Serialize, Deserialize)]
pub enum CoordinatorRequest {
    JoinServer,
    LeaveServer,
    ChangeName(String),
    ListChannels,
    JoinChannel(String, Process<ClientMessage>),
    LeaveChannel(String),
}

#[derive(Serialize, Deserialize)]
pub enum CoordinatorResponse {
    ServerJoined(Info),
    ServerLeft,
    NewUsername(String),
    ChannelList(Vec<(String, usize)>),
    ChannelJoined(Process<ChannelMessage>),
    ChannelDropped,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Info {
    pub username: String,
    pub total_clients: usize,
}

// A reference to a client that joined the server.
struct Client {
    link: Tag,
    username: String,
    // All channels that the client joined
    channels: HashSet<Process<ChannelMessage>>,
}

/// The coordinator keeps track of connected clients and active channels.
/// It's also in charge of assigning unique usernames to new clients.
pub fn coordinator_process(mailbox: Mailbox<Request<CoordinatorRequest, CoordinatorResponse>>) {
    let mut clients: HashMap<u128, Client> = HashMap::new();
    let mut channels: HashMap<String, (Process<ChannelMessage>, usize)> = HashMap::new();

    let mailbox = mailbox.catch_link_panic();

    loop {
        let message = mailbox.receive();

        // If the client dies remove it from the list and notify all channels.
        if let Message::Signal(link) = message {
            // Find the correct link
            let id = if let Some((id, client)) =
                clients.iter().find(|(_, client)| client.link == link)
            {
                // Notify all channels that the client is part of about leaving.
                client
                    .channels
                    .iter()
                    .for_each(|channel| channel.send(ChannelMessage::Drop(*id)));
                Some(*id)
            } else {
                None
            };
            if let Some(id) = id {
                clients.remove(&id);
            }
        }

        if let Message::Normal(message) = message {
            let message = message.unwrap();
            let data = message.data();

            match data {
                CoordinatorRequest::JoinServer => {
                    let client = message.sender();
                    let client_link = client.link();
                    // Tags are always unique inside of a process.
                    let client_username = format!("user_{}", client_link.id());

                    clients.insert(
                        client.id(),
                        Client {
                            link: client_link,
                            username: client_username.clone(),
                            channels: HashSet::new(),
                        },
                    );

                    message.reply(CoordinatorResponse::ServerJoined(Info {
                        username: client_username,
                        total_clients: clients.len(),
                    }))
                }

                CoordinatorRequest::LeaveServer => {
                    let client = message.sender();
                    let client_id = client.id();
                    // Notify all channels that the client is part of about leaving.
                    clients
                        .get(&client_id)
                        .unwrap()
                        .channels
                        .iter()
                        .for_each(|channel| channel.send(ChannelMessage::Drop(client_id)));
                    clients.remove(&client_id);
                    message.reply(CoordinatorResponse::ServerLeft)
                }

                CoordinatorRequest::ChangeName(new_name) => {
                    let client = message.sender();
                    // Check if username is taken
                    if let Some(old_name) =
                        clients.values().find(|client| client.username == *new_name)
                    {
                        // Don't change name if it's taken
                        message.reply(CoordinatorResponse::NewUsername(
                            old_name.username.to_string(),
                        ));
                    } else {
                        let new_name = new_name.to_string();
                        clients.get_mut(&client.id()).unwrap().username = new_name.clone();
                        message.reply(CoordinatorResponse::NewUsername(new_name));
                    }
                }

                CoordinatorRequest::ListChannels => {
                    let channels: Vec<(String, usize)> = channels
                        .iter()
                        .map(|(channel_name, (_, size))| (channel_name.clone(), *size))
                        .collect();
                    message.reply(CoordinatorResponse::ChannelList(channels));
                }

                CoordinatorRequest::JoinChannel(channel_name, client) => {
                    let channel = if let Some(exists) = channels.get_mut(channel_name) {
                        // Channel already exists
                        exists.1 += 1;
                        exists.0.send(ChannelMessage::Join(client.clone()));
                        exists.0.clone()
                    } else {
                        // Create a new channel process
                        let channel =
                            process::spawn_with(channel_name.clone(), channel_process).unwrap();
                        channel.send(ChannelMessage::Join(client.clone()));
                        channel
                    };
                    message.reply(CoordinatorResponse::ChannelJoined(channel));
                }
                CoordinatorRequest::LeaveChannel(channel_name) => {
                    let left = if let Some(exists) = channels.get_mut(channel_name) {
                        let client = message.sender();
                        exists.1 -= 1;
                        exists.0.send(ChannelMessage::Drop(client.id()));
                        exists.1
                    } else {
                        0 // If the channel doesn't exist, attempting to remove it will not have any effect
                    };
                    // If this was the last client remove the channel.
                    if left == 0 {
                        channels.remove(channel_name);
                    }
                }
            }
        }
    }
}
