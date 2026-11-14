pub struct FullScreen;

impl FullScreen {
    /// True for QUNS_BUSY (2), QUNS_RUNNING_D3D_FULL_SCREEN (3) and QUNS_PRESENTATION_MODE (4).
    pub fn is_full_screen(notification_state: i32) -> bool {
        matches!(notification_state, 2 | 3 | 4)
    }
}
