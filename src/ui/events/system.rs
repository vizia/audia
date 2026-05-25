#[derive(Clone, Debug)]
pub enum SystemEvent {
    Ready,
    StatusMessage(String),
    Error(String),
}
