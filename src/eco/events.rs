use crate::eco::clipboard::ClipboardData;
use crate::eco::device::EcoDevice;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum EcoEvent {
    DeviceDiscovered(Arc<EcoDevice>),
    DeviceConnected(Arc<EcoDevice>),
    DeviceDisconnected(Arc<EcoDevice>),
    DeviceTrusted(Arc<EcoDevice>),
    DeviceUntrusted(Arc<EcoDevice>),

    ClipboardChanged(Arc<ClipboardData>),
    ClipboardReceived(Arc<ClipboardData>, String),
    ClipboardApplied(Arc<ClipboardData>),

    PairingRequest(Arc<EcoDevice>),
    PairingAccepted(Arc<EcoDevice>),
    PairingRejected(Arc<EcoDevice>),

    Error(String),
    Synced(String),
}

pub type EventHandler = Arc<dyn Fn(EcoEvent) + Send + Sync>;

pub struct EventBus {
    handlers: std::sync::Mutex<Vec<EventHandler>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn subscribe(&self, handler: EventHandler) {
        self.handlers.lock().unwrap().push(handler);
    }

    pub fn emit(&self, event: EcoEvent) {
        let handlers = self.handlers.lock().unwrap();
        for handler in handlers.iter() {
            handler(event.clone());
        }
    }
}
