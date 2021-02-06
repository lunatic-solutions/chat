use std::collections::HashMap;

use lunatic::channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};

use crate::client::ClientMessage;

#[derive(Serialize, Deserialize)]
pub enum ChannelMessage {
    Join(String, Sender<ChannelMessage>),
    // channel name, sender, id_sender
    Subscribe(
        String,
        Sender<ClientMessage>,
        Sender<(u32, Vec<(String, String, String)>)>,
    ),
    // Channel name, username
    Joined(String, String),
    // Channel name, username, message
    Message(String, String, String, String),
    // Client with id left
    Unsubscribe(u32),
    // Assign id to client
    AssignId(u32),
}

pub fn channel_process((channel_name, channel_receiver): (String, Receiver<ChannelMessage>)) {
    let mut clients = HashMap::new();
    let mut id_generator: u32 = 0;
    let mut last_messages = Vec::new();
    loop {
        if let Ok(message) = channel_receiver.receive() {
            match message {
                ChannelMessage::Subscribe(name, client, id_sender) => {
                    clients.insert(id_generator, client.clone());
                    let _ = id_sender.send((id_generator, last_messages.clone()));
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

                ChannelMessage::Message(_, timestamp, name, message) => {
                    // Save
                    last_messages.push((timestamp.clone(), name.clone(), message.clone()));
                    // If too many last messages, drain
                    if last_messages.len() > 10 {
                        last_messages.drain(0..5);
                    }
                    // Broadcast
                    for (_id, client) in clients.iter() {
                        let _ =
                            client.send(ClientMessage::ChannelMessage(ChannelMessage::Message(
                                channel_name.clone(),
                                timestamp.clone(),
                                name.clone(),
                                message.clone(),
                            )));
                    }
                }
                _ => {}
            }
        } else {
            // Last use left channel.
            break;
        }
    }
}
