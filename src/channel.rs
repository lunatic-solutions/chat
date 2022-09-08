use std::collections::HashMap;

use lunatic::{abstract_process, process::ProcessRef};

use crate::client::{ClientProcess, ClientProcessHandler};

/// A channel dispatches messages to all clients that are part of it.
///
/// It also keeps the last few messages saved, so that it can bootstrap a new client that joins.
pub struct ChannelProcess {
    clients: HashMap<u64, ProcessRef<ClientProcess>>,
    last_messages: Vec<(String, String, String)>,
}

#[abstract_process(visibility = pub)]
impl ChannelProcess {
    #[init]
    fn init(_: ProcessRef<Self>, _name: String) -> Self {
        ChannelProcess {
            clients: HashMap::new(),
            last_messages: Vec::new(),
        }
    }

    /// join the channel.
    #[handle_message]
    fn join(&mut self, client: ProcessRef<ClientProcess>) {
        self.clients.insert(client.id(), client);
    }

    /// leave the channel.
    #[handle_message]
    fn leave(&mut self, client: ProcessRef<ClientProcess>) {
        self.clients.remove(&client.id());
    }

    /// Returns up to 10 last messages received by the channel.
    #[handle_request]
    fn get_last_messages(&mut self) -> Vec<(String, String, String)> {
        self.last_messages.clone()
    }

    /// Sent a new message to the channel.
    #[handle_message]
    fn broadcast_message(
        &mut self,
        channel: String,
        timestamp: String,
        name: String,
        message: String,
    ) {
        // Save
        self.last_messages
            .push((timestamp.clone(), name.clone(), message.clone()));
        // If too many last messages, drain
        if self.last_messages.len() > 10 {
            self.last_messages.drain(0..5);
        }
        // Broadcast message to all clients
        for (_id, client) in self.clients.iter() {
            client.receive_message(
                channel.clone(),
                timestamp.clone(),
                name.clone(),
                message.clone(),
            );
        }
    }
}
