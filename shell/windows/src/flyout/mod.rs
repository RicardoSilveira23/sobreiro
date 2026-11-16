//! The flyout: the card Sobreiro shows over the taskbar when the pill is clicked. It is drawn
//! from Core's `FlyoutModel`; `face` turns the model and the palette into what the .slint file
//! draws, `states` holds the fixed states the pictures and their tests use, and `pictures` is
//! `--flyout-pictures`; `toggle` is when the card is open and closed, and `window` is its window.
pub mod face;
pub mod pictures;
pub mod states;
pub mod toggle;
pub mod window;
