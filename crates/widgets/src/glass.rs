//! Glass: the frost tint over the blur. The light on its edge is
//! [`crate::specular`]'s, laid over every widget alike.

use crownui::prelude::Styled;

use crate::{Env, palette::MATERIAL, scale::ink};

/// Frosts `element`. On the board the wallpaper behind it is blurred; in the
/// gallery the tint stands alone.
pub fn glass<E: Styled>(element: E, env: Env) -> E {
    let tinted = element.bg(ink(env.ambient.palette, |palette| palette.frost));
    if env.is_frosted() {
        tinted.material(MATERIAL)
    } else {
        tinted
    }
}
