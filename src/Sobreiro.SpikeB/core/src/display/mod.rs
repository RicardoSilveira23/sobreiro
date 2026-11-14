//! Display: every word and number the pill, tray icon and flyout show, worked out from the limits.

mod duration;
mod flyout_model;
mod flyout_placement;
mod flyout_style;
mod glyph_fill;
mod limit_status;
mod pace;
mod pace_days;
mod pill_model;
mod pill_stale;
mod rings;
mod status_line_text;
mod tooltip_text;
mod when;
mod working_time;

pub use duration::Duration;
pub use flyout_model::{CopilotSection, FlyoutModel, LimitView};
pub use flyout_placement::{FlyoutPlacement, PixelRect, TaskbarEdge};
pub use flyout_style::FlyoutStyle;
pub use glyph_fill::GlyphFill;
pub use limit_status::{LimitStatus, StatusKind};
pub use pace::{Pace, PaceProjection};
pub use pace_days::PaceDays;
pub use pill_model::{PillCopilot, PillLine, PillModel};
pub use pill_stale::PillStale;
pub use rings::{Ring, RingsModel};
pub use status_line_text::StatusLineText;
pub use tooltip_text::TooltipText;
pub use when::{Clock, When};
pub use working_time::WorkingTime;
