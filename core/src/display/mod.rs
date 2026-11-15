//! What is shown, as text and as models for the pill and the flyout. Only the bridge's status
//! line and the two choices `config.json` keeps are here so far; the rest arrives with its own
//! area.
pub mod flyout_style;
pub mod pace_days;
pub mod status_line_text;

pub use flyout_style::FlyoutStyle;
pub use pace_days::PaceDays;
