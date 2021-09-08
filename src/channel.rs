use std::collections::HashMap;

use lunatic::{process::Process, Mailbox};
use serde::{Deserialize, Serialize};

use crate::client::ClientMessage;

#[derive(Serialize, Deserialize)]
pub enum ChannelMessage {
    Join(Process<ClientMessage>),
    // Channel name, username, message
    Message(String, String, String, String),
    // Client with id left
    Drop(u128),
}

pub fn channel_process(channel_name: String, mailbox: Mailbox<ChannelMessage>) {
    let mut clients = HashMap::new();
    let mut last_messages = Vec::new();

    loop {
        match mailbox.receive().unwrap() {
            ChannelMessage::Join(client) => {
                clients.insert(client.id(), client);
            }

            ChannelMessage::Drop(id) => {
                clients.remove(&id);
                if clients.is_empty() {
                    break;
                }
            }

            ChannelMessage::Message(_, timestamp, name, message) => {
                // Save
                last_messages.push((timestamp.clone(), name.clone(), message.clone()));
                // If too many last messages, drain
                if last_messages.len() > 10 {
                    last_messages.drain(0..5);
                }
                // Broadcast
                for (_id, client) in clients.iter() {
                    let _ = client.send(ClientMessage::Channel(ChannelMessage::Message(
                        channel_name.clone(),
                        timestamp.clone(),
                        name.clone(),
                        message.clone(),
                    )));
                }
            }
        }
    }
}
