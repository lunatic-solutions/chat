use std::collections::{HashMap, HashSet};

use crate::{
    channel::{ChannelProcess, ChannelProcessHandler},
    client::ClientProcess,
};

use lunatic::{
    abstract_process, host,
    process::{ProcessRef, StartProcess},
    supervisor::Supervisor,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Info {
    pub username: String,
    pub total_clients: usize,
}

// A reference to a client that joined the server.
struct Client {
    username: String,
    // All channels that the client joined
    channels: HashSet<ProcessRef<ChannelProcess>>,
}

/// The `CoordinatorSup` is supervising one global instance of the `CoordinatorProcess`.
pub struct CoordinatorSup;
impl Supervisor for CoordinatorSup {
    type Arg = String;
    type Children = CoordinatorProcess;

    fn init(config: &mut lunatic::supervisor::SupervisorConfig<Self>, name: Self::Arg) {
        // Always register the `CoordinatorProcess` under the name passed to the supervisor.
        config.children_args(((), Some(name)))
    }
}

/// The coordinator is in charge of keeping track of all connected clients and active channels.
///
/// A client will inform the coordinator that it joined the server, request a name change or join
/// a channel. The client can also query the coordinator for all currently active channels.
pub struct CoordinatorProcess {
    next_id: u64,
    clients: HashMap<u64, Client>,
    channels: HashMap<String, (ProcessRef<ChannelProcess>, usize)>,
}

#[abstract_process(visibility = pub)]
impl CoordinatorProcess {
    #[init]
    fn init(_: ProcessRef<Self>, _: ()) -> Self {
        // Coordinator shouldn't die when a client dies. This makes the link one-directional.
        unsafe { host::api::process::die_when_link_dies(0) };

        CoordinatorProcess {
            next_id: 0,
            clients: HashMap::new(),
            channels: HashMap::new(),
        }
    }

    /// Connect to the server.
    ///
    /// The coordinator will assign a unique `username` to the client and send back some server info,
    /// like the total count of connected clients.
    #[handle_request]
    fn join_server(&mut self, client: ProcessRef<ClientProcess>) -> Info {
        self.next_id += 1;
        let client_username = format!("user_{}", self.next_id);

        self.clients.insert(
            client.id(),
            Client {
                username: client_username.clone(),
                channels: HashSet::new(),
            },
        );

        Info {
            username: client_username,
            total_clients: self.clients.len(),
        }
    }

    /// leave the server.
    ///
    /// TODO: If the client fails unexpectedly, we need also to clean up after it.
    #[handle_message]
    fn leave_server(&mut self, client: ProcessRef<ClientProcess>) {
        self.clients
            .get(&client.id())
            .unwrap()
            .channels
            .iter()
            .for_each(|channel| channel.leave(client.clone()));
        self.clients.remove(&client.id());
    }

    /// Request for a name change by the client.
    #[handle_request]
    fn change_name(&mut self, client: ProcessRef<ClientProcess>, new_name: String) -> String {
        // Check if username is taken
        if let Some(old_name) = self
            .clients
            .values()
            .find(|client| client.username == *new_name)
        {
            // Don't change name if it's taken
            old_name.username.to_string()
        } else {
            self.clients.get_mut(&client.id()).unwrap().username = new_name.clone();
            new_name
        }
    }

    #[handle_request]
    fn list_channels(&mut self) -> Vec<(String, usize)> {
        self.channels
            .iter()
            .map(|(channel_name, (_, size))| (channel_name.clone(), *size))
            .collect()
    }

    #[handle_request]
    fn join_channel(
        &mut self,
        client: ProcessRef<ClientProcess>,
        channel: String,
    ) -> ProcessRef<ChannelProcess> {
        if let Some(exists) = self.channels.get_mut(&channel) {
            // Channel already exists
            exists.1 += 1;
            exists.0.join(client);
            exists.0.clone()
        } else {
            // Start a new channel process
            let channel_proc = ChannelProcess::start_link(channel.clone(), None);
            self.channels
                .insert(channel.clone(), (channel_proc.clone(), 1));
            channel_proc.join(client);
            channel_proc
        }
    }

    #[handle_message]
    fn leave_channel(&mut self, client: ProcessRef<ClientProcess>, channel: String) {
        let left = if let Some(exists) = self.channels.get_mut(&channel) {
            exists.0.leave(client);
            exists.1 -= 1;
            exists.1
        } else {
            // If the channel doesn't exist, attempting to remove it will not have any effect
            usize::MAX
        };
        // If this was the last client, shut down the channel and remove it.
        if left == 0 {
            let channel_proc = &self.channels.get(&channel).unwrap().0;
            channel_proc.shutdown();
            self.channels.remove(&channel);
        }
    }
}
