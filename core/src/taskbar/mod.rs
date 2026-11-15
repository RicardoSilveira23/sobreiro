//! Where the pill goes on the taskbar and when to ask where the taskbar's parts are: whether
//! a window is full screen, the pill's place beside the tray icons, what showing it does to its
//! window, the anchors read from the rectangles UI Automation gives, and the schedule of those
//! questions. The shell asks the system; the rules are here.
pub mod full_screen;
pub mod pill_placement;
pub mod pill_show;
pub mod taskbar_layout;
pub mod taskbar_query_schedule;
