//! A pair of earbuds over a violet horizon, with the charge they report and
//! what they are called.

mod readout;

use crownui::prelude::{Cx, Size, Styled, View, bind, rgb, zstack};
use serde::{Deserialize, Serialize};

use crate::{
    Built, Env, Shape, art,
    palette::MATERIAL,
    scale::Scale,
    shape::{TILE, rounding},
};
use crownhome_board::WidgetSpec;
use crownhome_services::earbuds::{Earbuds, follow};

pub const KIND: &str = "earbuds";

/// The frame of a 2×3 tile, the size the widget is drawn for.
const DESIGN: (f32, f32) = (TILE.width, 382.0);
const NIGHT: u32 = 0x151417;
const SCENE: &[u8] = include_bytes!("../../assets/earbuds/scene.svg");

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {}

fn measure(size: Size) -> f32 {
    (size.width / DESIGN.0).min(size.height / DESIGN.1)
}

pub fn shape(size: Size) -> Shape {
    Shape::filling(size, rounding(size))
}

fn sample() -> Earbuds {
    Earbuds {
        name: "Studio Buds".to_owned(),
        connected: true,
        battery: Some(80),
    }
}

pub fn view(cx: &mut Cx, spec: &WidgetSpec, env: Env) -> Built {
    let _: Settings = spec.decode()?;
    let earbuds = cx.signal(None::<Earbuds>);
    if env.is_live() {
        follow(cx, earbuds);
    } else {
        earbuds.set(cx, Some(sample()));
    }
    let scale = Scale::new(cx, env.size, measure);
    let size = env.size;
    let card = zstack((
        art::backdrop(cx, SCENE, size),
        readout::view(earbuds, scale),
    ))
    .relative()
    .w_full()
    .h_full()
    .bg(rgb(NIGHT))
    .rounded(bind(move |runtime| rounding(size.get(runtime))))
    .overflow_hidden();
    Ok(if env.is_live() {
        card.material(MATERIAL).boxed()
    } else {
        card.boxed()
    })
}
