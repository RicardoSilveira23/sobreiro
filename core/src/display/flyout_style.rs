//! The looks the flyout can take.

/// How the flyout draws a limit. `config.json` keeps the user's choice under `style`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlyoutStyle {
    Bars,
    Battery,
    Thermometer,
}
