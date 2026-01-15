//! Dialog windows for RemoteDesk

pub mod connection_request;
pub mod password;
pub mod peer_id;
pub mod settings;

pub use connection_request::ConnectionRequestDialog;
pub use password::PasswordDialog;
pub use peer_id::PeerIdDialog;
pub use settings::SettingsDialog;
