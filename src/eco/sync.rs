use crate::eco::clipboard::ClipboardData;
use crate::eco::device::EcoDevice;
use crate::eco::errors::EcoResult;
use crate::eco::events::EventBus;

/// Synchronization manager — skeleton for future cross-device sync features.
/// Currently only handles clipboard sync routing.
pub struct SyncManager {
    event_bus: std::sync::Arc<EventBus>,
}

impl SyncManager {
    pub fn new(event_bus: std::sync::Arc<EventBus>) -> Self {
        Self { event_bus }
    }

    pub async fn start(&self) -> EcoResult<()> {
        Ok(())
    }

    pub async fn shutdown(&self) {
    }

    pub async fn sync_clipboard(&self, data: ClipboardData, _source: EcoDevice) {
        use crate::eco::events::EcoEvent;
        let arc = std::sync::Arc::new(data);
        self.event_bus.emit(EcoEvent::ClipboardReceived(arc, String::new()));
    }
}
