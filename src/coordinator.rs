use std::collections::{HashMap, HashSet};

use crate::{
    channel::{self, ChannelProcess},
    client::ClientProcess,
};

use lunatic::{
    host,
    process::{AbstractProcess, Message, ProcessMessage, ProcessRef, ProcessRequest, StartProcess},
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
    clients: HashMap<u128, Client>,
    channels: HashMap<String, (ProcessRef<ChannelProcess>, usize)>,
}
impl AbstractProcess for CoordinatorProcess {
    type Arg = ();
    type State = Self;

    fn init(_: ProcessRef<Self>, _: Self::Arg) -> Self::State {
        // Coordinator shouldn't die when a client dies. This makes the link one-directional.
        unsafe { host::api::process::die_when_link_dies(0) };

        CoordinatorProcess {
            next_id: 0,
            clients: HashMap::new(),
            channels: HashMap::new(),
        }
    }
}

/// A `JoinServer` is sent out by a client that just connected to the server.
///
/// The coordinator will assign a unique `username` to the client and send back some server info,
/// like the total count of connected clients.
#[derive(Serialize, Deserialize)]
pub struct JoinServer(pub ProcessRef<ClientProcess>);
impl ProcessRequest<JoinServer> for CoordinatorProcess {
    type Response = Info;

    fn handle(state: &mut Self::State, JoinServer(client): JoinServer) -> Self::Response {
        state.next_id += 1;
        let client_username = format!("user_{}", state.next_id);

        state.clients.insert(
            client.uuid(),
            Client {
                username: client_username.clone(),
                channels: HashSet::new(),
            },
        );

        Info {
            username: client_username,
            total_clients: state.clients.len(),
        }
    }
}

/// Sent by client when leaving the server.
///
/// TODO: If the client fails unexpectedly, we need also to clean up after it.
#[derive(Serialize, Deserialize)]
pub struct LeaveServer(pub ProcessRef<ClientProcess>);
impl ProcessMessage<LeaveServer> for CoordinatorProcess {
    fn handle(state: &mut Self::State, LeaveServer(client): LeaveServer) {
        state
            .clients
            .get(&client.uuid())
            .unwrap()
            .channels
            .iter()
            .for_each(|channel| channel.send(channel::Leave(client.clone())));
        state.clients.remove(&client.uuid());
    }
}

/// Request for a name change by the client.
#[derive(Serialize, Deserialize)]
pub struct ChangeName(pub ProcessRef<ClientProcess>, pub String);
impl ProcessRequest<ChangeName> for CoordinatorProcess {
    type Response = String;

    fn handle(state: &mut Self::State, ChangeName(client, new_name): ChangeName) -> Self::Response {
        // Check if username is taken
        if let Some(old_name) = state
            .clients
            .values()
            .find(|client| client.username == *new_name)
        {
            // Don't change name if it's taken
            old_name.username.to_string()
        } else {
            state.clients.get_mut(&client.uuid()).unwrap().username = new_name.clone();
            new_name
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ListChannels;
impl ProcessRequest<ListChannels> for CoordinatorProcess {
    type Response = Vec<(String, usize)>;

    fn handle(state: &mut Self::State, _: ListChannels) -> Self::Response {
        state
            .channels
            .iter()
            .map(|(channel_name, (_, size))| (channel_name.clone(), *size))
            .collect()
    }
}

#[derive(Serialize, Deserialize)]
pub struct JoinChannel(pub ProcessRef<ClientProcess>, pub String);
impl ProcessRequest<JoinChannel> for CoordinatorProcess {
    type Response = ProcessRef<ChannelProcess>;

    fn handle(
        state: &mut Self::State,
        JoinChannel(client, channel): JoinChannel,
    ) -> Self::Response {
        if let Some(exists) = state.channels.get_mut(&channel) {
            // Channel already exists
            exists.1 += 1;
            exists.0.send(channel::Join(client));
            exists.0.clone()
        } else {
            // Start a new channel process
            let channel_proc = ChannelProcess::start_link(channel.clone(), None);
            state
                .channels
                .insert(channel.clone(), (channel_proc.clone(), 1));
            channel_proc.send(channel::Join(client));
            channel_proc
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct LeaveChannel(pub ProcessRef<ClientProcess>, pub String);
impl ProcessMessage<LeaveChannel> for CoordinatorProcess {
    fn handle(state: &mut Self::State, LeaveChannel(client, channel): LeaveChannel) {
        let left = if let Some(exists) = state.channels.get_mut(&channel) {
            exists.0.send(channel::Leave(client));
            exists.1 -= 1;
            exists.1
        } else {
            // If the channel doesn't exist, attempting to remove it will not have any effect
            usize::MAX
        };
        // If this was the last client, shut down the channel and remove it.
        if left == 0 {
            let channel_proc = &state.channels.get(&channel).unwrap().0;
            channel_proc.shutdown();
            state.channels.remove(&channel);
        }
    }
}
