use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

pub(crate) static ASSISTANT_STATE: once_cell::sync::Lazy<Arc<Mutex<AssistantState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(AssistantState::default())));

pub(crate) static NLU_READY: AtomicBool = AtomicBool::new(false);

pub(crate) static ONLINE_FAIL_COUNT: AtomicU32 = AtomicU32::new(0);

pub(crate) static UI_PANEL_STATE: once_cell::sync::Lazy<Arc<Mutex<UiPanelState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(UiPanelState::default())));

#[derive(Default)]
pub(crate) struct UiPanelState {
    pub(crate) show_fastswap: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct AssistantState {
    pub(crate) is_initialized: bool,
    pub(crate) is_listening: bool,
    pub(crate) is_awake: bool,
    pub(crate) current_status: String,
    pub(crate) last_command: String,
    pub(crate) logs: Vec<(String, LogLevel)>,
    pub(crate) running_apps: Vec<String>,
    pub(crate) setup_in_progress: bool,
}

impl Default for AssistantState {
    fn default() -> Self {
        Self {
            is_initialized: false,
            is_listening: false,
            is_awake: false,
            current_status: "Running First-Time Setup...".to_string(),
            last_command: String::new(),
            logs: vec![(
                "Welcome to IGRIS - Your Voice Assistant".to_string(),
                LogLevel::Info,
            )],
            running_apps: Vec::new(),
            setup_in_progress: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

pub(crate) fn update_status(status: &str) {
    let mut state = ASSISTANT_STATE.lock().unwrap();
    state.current_status = status.to_string();
}

pub(crate) fn add_log(message: &str, level: LogLevel) {
    let mut state = ASSISTANT_STATE.lock().unwrap();
    state.logs.push((message.to_string(), level));
    if state.logs.len() > 100 {
        state.logs.remove(0);
    }
}
