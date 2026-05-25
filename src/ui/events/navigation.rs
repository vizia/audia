use crate::ui::model_data::{CenterPage, RightPanelPage};

#[derive(Clone, Debug)]
pub enum CenterEvents {
    NavigateTo(CenterPage),
    NavigateBack,
    NavigateForward,
}

#[derive(Clone, Debug)]
pub enum RightPanelEvents {
    NavigateTo(RightPanelPage),
}
