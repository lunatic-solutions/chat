use std::collections::HashMap;

use lunatic::channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};

use crate::client::ClientMessage;

#[derive(Serialize, Deserialize)]
pub enum ChannelMessage {
    Join(String, Sender<ChannelMessage>),
    // channel name, sender, id_sender
    Subscribe(String, Sender<ClientMessage>, Sender<u32>),
    // Channel name, username
    Joined(String, String),
    // Channel name, username, message
    Message(String, String, String),
    // Client with id left
    Unsubscribe(u32),
    // Assign id to client
    AssignId(u32),
}

pub fn channel_process((channel_name, channel_receiver): (String, Receiver<ChannelMessage>)) {
    let mut clients = HashMap::new();
    let mut id_generator: u32 = 0;
    loop {
        match channel_receiver.receive().unwrap() {
            ChannelMessage::Subscribe(name, client, id_sender) => {
                clients.insert(id_generator, client.clone());
                let _ = id_sender.send(id_generator);
                id_generator += 1;

                // Notify all participants about the new join
                for (_id, client) in clients.iter() {
                    let _ = client.send(ClientMessage::ChannelMessage(ChannelMessage::Joined(
                        channel_name.clone(),
                        name.clone(),
                    )));
                }
            }

            ChannelMessage::Unsubscribe(id) => {
                clients.remove(&id);
                if clients.is_empty() {
                    break;
                }
            }

            ChannelMessage::Message(_, name, message) => {
                // Broadcast
                for (_id, client) in clients.iter() {
                    let _ = client.send(ClientMessage::ChannelMessage(ChannelMessage::Message(
                        channel_name.clone(),
                        name.clone(),
                        message.clone(),
                    )));
                }
            }
            _ => {}
        }
    }
}
