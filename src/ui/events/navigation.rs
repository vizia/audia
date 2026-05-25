use crate::ui::model_data::{CenterPage, RightPanelPage};

#[derive(Clone, Debug)]
pub enum CenterPanelEvent {
    NavigateTo(CenterPage),
    NavigateBack,
    NavigateForward,
}

#[derive(Clone, Debug)]
pub enum RightPanelEvent {
    NavigateTo(RightPanelPage),
}
