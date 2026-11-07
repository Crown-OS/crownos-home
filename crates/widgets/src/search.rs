//! A clear glass search field that opens the launcher.

use crownui::prelude::{Cx, Listeners, Size, Styled, View, bind, hstack, icon, text, vstack};
use serde::{Deserialize, Serialize};

use crate::{
    Built, Env, Shape,
    glass::glass,
    scale::{Scale, ink},
};
use crownhome_board::WidgetSpec;
use crownhome_services::crownpad;

pub const KIND: &str = "search";

const DESIGN_HEIGHT: f32 = 72.0;
const FILL: f32 = 0.72;
const LENS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2.2" stroke-linecap="round"><circle cx="10.5" cy="10.5" r="7"/><path d="M15.6 15.6 21 21"/></svg>"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    #[serde(default = "default_placeholder")]
    pub placeholder: String,
}

fn default_placeholder() -> String {
    "Search".to_owned()
}

pub fn shape(size: Size) -> Shape {
    let height = (size.height * FILL).min(size.width);
    Shape::band(size, height, height / 2.0)
}

pub fn view(cx: &mut Cx, spec: &WidgetSpec, env: Env) -> Built {
    let settings: Settings = spec.decode()?;
    let palette = env.ambient.palette;
    let pill_height = cx.memo(move |runtime| {
        let size = env.size.get(runtime);
        (size.height * FILL).min(size.width)
    });
    let scale = Scale::new(cx, env.size, |size| {
        (size.height * FILL).min(size.width) / DESIGN_HEIGHT
    });
    let lens = icon(LENS, ink(palette, |palette| palette.ink_on_clear))
        .w(scale.px(28.0))
        .h(scale.px(28.0))
        .shrink(0.0);
    let placeholder = text(settings.placeholder)
        .text_size(scale.px(25.0))
        .truncate()
        .text_color(ink(palette, |palette| palette.ink_on_clear));
    let pill = hstack((lens, placeholder))
        .items_center()
        .gap(scale.px(14.0))
        .px(scale.px(22.0))
        .w_full()
        .h(pill_height)
        .rounded(bind(move |runtime| pill_height.get(runtime) / 2.0));
    let field = vstack(glass(pill, env, |palette| palette.clear))
        .w_full()
        .h_full()
        .justify_center();
    if !env.is_live() {
        return Ok(field.boxed());
    }
    Ok(field
        .on_click(|cx, _| {
            cx.spawn(crownpad::show(), |_, ()| {});
        })
        .boxed())
}
