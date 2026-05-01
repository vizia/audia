mod messages;
mod oauth;
mod playback;
mod spotify;
mod storage;
mod worker;

mod ui;

fn main() -> Result<(), vizia::ApplicationError> {
    ui::run()
}
