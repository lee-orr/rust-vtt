#[derive(Default)]
pub struct CommunicationResource {
    pub state: CommunicationState,
    pub running: bool,
}

pub enum CommunicationState {
    None,
    Server { port: u16 },
    Client { url: String },
}

impl Default for CommunicationState {
    fn default() -> Self {
        CommunicationState::None
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ServerState {
    Closed,
    Open,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClientState {
    Closed,
    Connecting,
    Open,
}

pub struct CloseServerEvent;

#[derive(Default)]
pub struct PendingMessage {
    pub value: String,
}

#[derive(Default)]
pub struct ReceivedMessages {
    pub messages: Vec<(usize, String)>,
}

pub struct SendMessageEvent {
    pub value: String,
}
