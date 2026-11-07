//! The folder's name and size across the bottom of the pane, and its button:
//! add files while it is closed, close it while it is open.

use std::path::PathBuf;

use crownui::prelude::{
    Color, CursorIcon, Listeners, Memo, Role, Semantic, Signal, Spring, Styled, View, bind, hstack,
    icon, keyed, spacer, text, vstack,
};

use super::{State, delivery, front::Tone};
use crate::scale::Scale;
use crownhome_services::folder::{self, Listing};

const PLUS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2.6" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>"#;
const CLOSE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2.4" stroke-linecap="round"><path d="M6.5 6.5l11 11M17.5 6.5l-11 11"/></svg>"#;
const BUTTON_EDGE: f32 = 32.0;
const GLYPH_EDGE: f32 = 15.0;
const INSET: (f32, f32) = (16.0, 15.0);
const BUTTON_FILL: Color = Color::new(0.07, 0.07, 0.08, 1.0);

/// `12 items`, `1 item` or `Empty`; nothing while the folder is being read.
fn items(count: Option<usize>) -> String {
    match count {
        None => String::new(),
        Some(0) => "Empty".to_owned(),
        Some(1) => "1 item".to_owned(),
        Some(count) => format!("{count} items"),
    }
}

/// Asks for files to copy into `dir`, then plays them arriving.
fn add(
    cx: &mut crownui::prelude::Cx,
    dir: PathBuf,
    listing: Signal<Option<Listing>>,
    state: State,
) {
    let into = dir.clone();
    cx.spawn(folder::add_files(dir), move |cx, names| {
        delivery::receive(cx, state, into, listing, names, delivery::PICKED_HOVER);
    });
}

fn button(
    dir: PathBuf,
    listing: Signal<Option<Listing>>,
    tone: Memo<Tone>,
    state: State,
    scale: Scale,
) -> impl View {
    let glyph = keyed(
        move |runtime| state.opened.get(runtime),
        move |_, opened| {
            let ink = bind(move |runtime| match opened {
                true => tone.get(runtime).ink(),
                false => Color::new(1.0, 1.0, 1.0, 1.0),
            });
            icon(if opened { CLOSE } else { PLUS }, ink)
                .w(scale.px(GLYPH_EDGE))
                .h(scale.px(GLYPH_EDGE))
        },
    );
    vstack(glyph)
        .w(scale.px(BUTTON_EDGE))
        .h(scale.px(BUTTON_EDGE))
        .rounded(scale.px(BUTTON_EDGE / 2.0))
        .items_center()
        .justify_center()
        .shrink(0.0)
        .bg(bind(move |runtime| match state.opened.get(runtime) {
            true => tone.get(runtime).ink().with_alpha_factor(0.08),
            false => BUTTON_FILL,
        }))
        .transition(Spring::snappy())
        .hover(|style| style.scale(1.08))
        .cursor(CursorIcon::Pointer)
        .role(Role::Button)
        .label("Add Files or Close")
        .pointer_events_auto()
        .on_click(move |cx, event| {
            event.stop_propagation();
            if state.opened.get(cx) {
                state.opened.set(cx, false);
            } else {
                add(cx, dir.clone(), listing, state);
            }
        })
}

pub fn view(
    title: String,
    count: Memo<Option<usize>>,
    dir: PathBuf,
    listing: Signal<Option<Listing>>,
    tone: Memo<Tone>,
    state: State,
    scale: Scale,
) -> impl View + Styled {
    let name = text(title)
        .text_size(scale.px(15.0))
        .font_weight(600)
        .tracking(scale.px(-0.2))
        .line_clamp(1)
        .text_color(bind(move |runtime| tone.get(runtime).ink()));
    let size = text(bind(move |runtime| items(count.get(runtime))))
        .text_size(scale.px(11.5))
        .text_color(bind(move |runtime| tone.get(runtime).ink_secondary()));
    let row = hstack((
        vstack((name, size))
            .gap(scale.px(1.0))
            .shrink(1.0)
            .min_w(0.0),
        spacer(),
        button(dir, listing, tone, state, scale),
    ))
    .absolute()
    .left(scale.px(INSET.0))
    .right(scale.px(INSET.1))
    .bottom(scale.px(INSET.1))
    .items_end()
    .gap(scale.px(8.0));
    vstack(row).pointer_events_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_size_reads_naturally() {
        assert_eq!(items(Some(0)), "Empty");
        assert_eq!(items(Some(1)), "1 item");
        assert_eq!(items(Some(12)), "12 items");
        assert_eq!(items(None), "");
    }
}
