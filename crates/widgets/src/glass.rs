//! Glass: the compositor's blur under a tint and a lit rim.

use crownui::prelude::Styled;

use crate::{
    Env, Finish,
    palette::{Glass, MATERIAL, Palette},
    scale::ink,
};

const RIM: f32 = 1.0;

/// Frosts `element` with the glass `pick` chooses. On the board the
/// compositor blurs what is behind it; in the gallery the tint stands alone.
pub fn glass<E: Styled>(element: E, env: Env, pick: fn(&Palette) -> Glass) -> E {
    let palette = env.ambient.palette;
    let tinted = element
        .bg(ink(palette, move |palette| pick(palette).tint))
        .border(RIM)
        .border_color(ink(palette, move |palette| pick(palette).rim));
    match env.finish {
        Finish::Live => tinted.material(MATERIAL),
        Finish::Sample => tinted,
    }
}
