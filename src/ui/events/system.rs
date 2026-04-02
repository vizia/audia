#[derive(Clone, Debug)]
pub enum SystemAppEvent {
    Ready,
    StatusMessage(String),
    Error(String),
}
