#[derive(Clone, Debug)]
pub enum SystemEvents {
    Ready,
    StatusMessage(String),
    Error(String),
}
