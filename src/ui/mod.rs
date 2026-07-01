// src/ui/mod.rs - UI Components

pub mod settings;
pub mod search_results;
pub mod camera_panel;
pub mod presentation;
pub mod menu_button;
pub mod fastswap_panel;
pub mod incoming_transfer_popup;
pub mod notification_panel;

pub use settings::{SettingsPanel, SettingsButton};
pub use search_results::{SearchResultsPanel, SearchResultItem};
pub use camera_panel::CameraPanel;
pub use presentation::{PresentationPanel, start_presentation, stop_presentation, is_presentation_active, is_presentation_open};
pub use menu_button::MenuButton;
pub use fastswap_panel::FastSwapPanel;
pub use incoming_transfer_popup::IncomingTransferPopup;
pub use notification_panel::NotificationPanel;
