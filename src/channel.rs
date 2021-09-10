use std::collections::HashMap;

use lunatic::{process::Process, Mailbox};
use serde::{Deserialize, Serialize};

use crate::client::{Channel, ClientMessage};

#[derive(Serialize, Deserialize)]
pub enum ChannelMessage {
    Join(Process<ClientMessage>),
    /// Client sent message to the channel
    Message(String, String, String),
    /// Client requested last messages
    LastMessages(Process<ClientMessage>),
    /// Client with id left
    Drop(u128),
}

pub fn channel_process(channel_name: String, mailbox: Mailbox<ChannelMessage>) {
    let mut clients = HashMap::new();
    let mut last_messages = Vec::new();

    loop {
        match mailbox.receive_with_tag().unwrap() {
            (ChannelMessage::Join(client), _) => {
                clients.insert(client.id(), client);
            }

            (ChannelMessage::LastMessages(client), tag) => client.tag_send(
                tag,
                ClientMessage::Channel(Channel::LastMessages(last_messages.clone())),
            ),

            (ChannelMessage::Drop(id), _) => {
                clients.remove(&id);
                if clients.is_empty() {
                    break;
                }
            }

            (ChannelMessage::Message(timestamp, name, message), _) => {
                // Save
                last_messages.push((timestamp.clone(), name.clone(), message.clone()));
                // If too many last messages, drain
                if last_messages.len() > 10 {
                    last_messages.drain(0..5);
                }
                // Broadcast
                for (_id, client) in clients.iter() {
                    let _ = client.send(ClientMessage::Channel(Channel::Message(
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
