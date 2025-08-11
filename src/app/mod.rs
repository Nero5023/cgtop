use crate::collection::CGroupMetrics;
use crossbeam::channel::Receiver;
use std::time::Instant;

pub struct App {
    pub cgroup_data: CGroupData,
    pub ui_state: UiState,
    pub config: Config,
    pub filters: FilterState,
    pub input_receiver: Option<Receiver<InputEvent>>,
    pub data_receiver: Option<Receiver<CGroupMetrics>>,
}

#[derive(Default)]
pub struct CGroupData {
    pub metrics: Option<Box<CGroupMetrics>>,
    pub last_update: Option<Instant>,
}

#[derive(Default)]
pub struct UiState {
    pub current_tab: usize,
    pub tree_expanded: Vec<String>,
    pub selected_cgroup: Option<String>,
    pub scroll_offset: usize,
}

#[derive(Default)]
pub struct Config {
    pub update_interval_ms: u64,
    pub data_retention_seconds: u64,
}

#[derive(Default)]
pub struct FilterState {
    pub name_filter: String,
    pub show_empty_cgroups: bool,
}

pub enum InputEvent {
    Key(crossterm::event::KeyEvent),
    Resize(u16, u16),
    Quit,
}

impl App {
    pub fn new() -> Self {
        Self {
            cgroup_data: CGroupData::default(),
            ui_state: UiState::default(),
            config: Config::default(),
            filters: FilterState::default(),
            input_receiver: None,
            data_receiver: None,
        }
    }

    pub fn set_channels(
        &mut self,
        input_rx: Receiver<InputEvent>,
        data_rx: Receiver<CGroupMetrics>,
    ) {
        self.input_receiver = Some(input_rx);
        self.data_receiver = Some(data_rx);
    }
}
