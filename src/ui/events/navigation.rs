use crate::ui::model_data::{CenterPage, RightPanelPage};

#[derive(Clone, Debug)]
pub enum CenterUiEvent {
    NavigateTo(CenterPage),
    NavigateBack,
    NavigateForward,
}

#[derive(Clone, Debug)]
pub enum RightPanelUiEvent {
    NavigateTo(RightPanelPage),
}
