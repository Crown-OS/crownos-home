//! What a widget is built against.

use crownui::prelude::{Memo, Signal, Size};
use jiff::Zoned;

use crate::palette::Palette;
use crownhome_services::plugins::Plugins;

/// What every widget on the desktop shares.
#[derive(Clone, Copy)]
pub struct Ambient {
    /// The time, moved on every second.
    pub now: Signal<Zoned>,
    pub palette: Memo<Palette>,
    pub plugins: Plugins,
}

/// How a widget is finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    /// On the board: frosted by the compositor, fed live data, clickable.
    Live,
    /// In the gallery: tinted glass only, sample data, no background work.
    Sample,
}

#[derive(Clone, Copy)]
pub struct Env {
    pub ambient: Ambient,
    /// The frame the widget fills.
    pub size: Memo<Size>,
    pub finish: Finish,
}

impl Env {
    pub fn is_live(&self) -> bool {
        self.finish == Finish::Live
    }
}
