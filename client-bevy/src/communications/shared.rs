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