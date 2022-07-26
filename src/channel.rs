use std::collections::HashMap;

use lunatic::process::{AbstractProcess, Message, MessageHandler, ProcessRef, RequestHandler};
use serde::{Deserialize, Serialize};

use crate::client::{ChannelMessage, ClientProcess};

/// A channel dispatches messages to all clients that are part of it.
///
/// It also keeps the last few messages saved, so that it can bootstrap a new client that joins.
pub struct ChannelProcess {
    clients: HashMap<u64, ProcessRef<ClientProcess>>,
    last_messages: Vec<(String, String, String)>,
}

impl AbstractProcess for ChannelProcess {
    type Arg = String;
    type State = Self;

    fn init(_: ProcessRef<Self>, _name: Self::Arg) -> Self::State {
        ChannelProcess {
            clients: HashMap::new(),
            last_messages: Vec::new(),
        }
    }
}

/// A `Join` message is sent by clients that would like to join the channel.
///
/// It contains a reference to the client.
#[derive(Serialize, Deserialize)]
pub struct Join(pub ProcessRef<ClientProcess>);
impl MessageHandler<Join> for ChannelProcess {
    fn handle(state: &mut Self::State, Join(client): Join) {
        state.clients.insert(client.id(), client);
    }
}

/// Returns up to 10 last messages received by the channel.
#[derive(Serialize, Deserialize)]
pub struct LastMessages;
impl RequestHandler<LastMessages> for ChannelProcess {
    type Response = Vec<(String, String, String)>;

    fn handle(state: &mut Self::State, _: LastMessages) -> Self::Response {
        state.last_messages.clone()
    }
}

/// A `Leave` message is sent by clients that would like to leave the channel.
///
/// It contains a reference to the client.
#[derive(Serialize, Deserialize)]
pub struct Leave(pub ProcessRef<ClientProcess>);
impl MessageHandler<Leave> for ChannelProcess {
    fn handle(state: &mut Self::State, Leave(client): Leave) {
        state.clients.remove(&client.id());
    }
}

/// A new message sent to the channel.
///
/// It contains ("", timestamp, name, message content).
///
/// The first argument is reserved for the name
impl MessageHandler<ChannelMessage> for ChannelProcess {
    fn handle(state: &mut Self::State, message: ChannelMessage) {
        // Save
        state
            .last_messages
            .push((message.1.clone(), message.2.clone(), message.3.clone()));
        // If too many last messages, drain
        if state.last_messages.len() > 10 {
            state.last_messages.drain(0..5);
        }
        // Broadcast message to all clients
        for (_id, client) in state.clients.iter() {
            let _ = client.send(message.clone());
        }
    }
}
