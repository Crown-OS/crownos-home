//! A record in a white case: it spins while music plays, rises out of its
//! sleeve for every new track, and the sleeve carries the transport buttons.

mod record;
mod transport;

use crownui::prelude::{Cx, Memo, Size, Styled, View, vstack, zstack};
use serde::{Deserialize, Serialize};

use crate::{
    Built, Env, Shape, art, music,
    palette::MATERIAL,
    scale::Scale,
    shape::{ROUNDING, TILE},
};
use crownhome_board::WidgetSpec;
use crownhome_services::music::{Playback, follow};

pub const KIND: &str = "vinyl";

/// The artwork is drawn on a square this many units wide.
const DESIGN_EDGE: f32 = 1000.0;
const CORNER: f32 = ROUNDING * DESIGN_EDGE / TILE.width;
const SHELL: &[u8] = include_bytes!("../../assets/vinyl/shell.svg");
const SLEEVE: &[u8] = include_bytes!("../../assets/vinyl/sleeve.svg");

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {}

fn edge(size: Size) -> f32 {
    size.width.min(size.height).floor().max(1.0)
}

fn measure(size: Size) -> f32 {
    edge(size) / DESIGN_EDGE
}

pub fn shape(size: Size) -> Shape {
    let edge = edge(size);
    Shape::centred_square(size, edge, edge * CORNER / DESIGN_EDGE)
}

fn layer(cx: &mut Cx, svg: &'static [u8], edge: Memo<f32>) -> impl View + use<> {
    vstack(art::picture(cx, move |_| svg, edge))
        .absolute()
        .inset(0.0)
        .pointer_events_none()
}

pub fn view(cx: &mut Cx, spec: &WidgetSpec, env: Env) -> Built {
    let _: Settings = spec.decode()?;
    let playback = cx.signal(None::<Playback>);
    if env.is_live() {
        follow(cx, playback);
    } else {
        playback.set(cx, Some(music::sample()));
    }
    let scale = Scale::new(cx, env.size, measure);
    let edge = scale.length(cx, DESIGN_EDGE);
    let case = zstack((
        layer(cx, SHELL, edge),
        record::view(cx, playback, scale),
        layer(cx, SLEEVE, edge),
        transport::view(playback, env, scale),
    ))
    .relative()
    .w(edge)
    .h(edge)
    .rounded(scale.px(CORNER))
    .overflow_hidden();
    let case = if env.is_live() {
        case.material(MATERIAL)
    } else {
        case
    };
    Ok(vstack(case)
        .w_full()
        .h_full()
        .items_center()
        .justify_center()
        .boxed())
}
