//! Taskbar: where the pill goes, and when the shell asks the taskbar about itself.

mod full_screen;
mod pill_placement;
mod pill_show;
mod taskbar_layout;
mod taskbar_query_schedule;

pub use full_screen::FullScreen;
pub use pill_placement::{PillHidden, PillPlace, PillPlacement};
pub use pill_show::{PillShow, PillShowStep, ZOrder};
pub use taskbar_layout::{TaskbarAnchors, TaskbarLayout};
pub use taskbar_query_schedule::TaskbarQuerySchedule;
