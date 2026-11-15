//! What is shown, as text and as models for the pill and the flyout: the bridge's status line,
//! the two choices `config.json` keeps, how times are written, the pace of a limit and the
//! badge it gets, the rings, when the pill's figures count as stopped, where the flyout
//! opens, and the models the flyout, the pill and the tray tooltip show. The shell hands in the
//! time zone (`Zone`) and the way the user writes a clock (`ClockFormat`), since Core asks the
//! system neither.
pub mod clock_format;
pub mod duration;
pub mod flyout_model;
pub mod flyout_placement;
pub mod flyout_style;
pub mod glyph_fill;
pub mod limit_status;
pub mod pace;
pub mod pace_days;
pub mod pill_model;
pub mod pill_stale;
pub mod rings;
pub mod status_line_text;
pub mod tooltip_text;
pub mod when;
pub mod working_time;
pub mod zone;

pub use clock_format::ClockFormat;
pub use flyout_style::FlyoutStyle;
pub use pace_days::PaceDays;
pub use zone::{Zone, ZoneChange};
