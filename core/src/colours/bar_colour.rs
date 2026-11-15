//! The colour of the bars.

/// Which colour the bars take: Sobreiro's purple, or the accent colour the user chose in
/// Windows. `config.json` keeps the choice under `bar_colour`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarColourOption {
    Purple,
    Accent,
}
