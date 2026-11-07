//! The widget gallery: a glass pane along the right edge. A card dragged onto
//! the desktop lands where it is dropped, a click adds it in the first free
//! space, and a widget dragged back onto the pane is put away.

use crownui::prelude::{
    AnyView, Color, CursorIcon, Cx, Listeners, Point, Role, Runtime, Semantic, Size, Spring,
    Styled, View, WHITE, hstack, keyed, scroll_view, show, spacer, text, vstack,
};

use crate::ui::{carry::carries, desk::Desk, gesture::Grip};
use crownhome_widgets::palette::{MATERIAL, Palette};
use crownhome_widgets::{
    self, Env, Finish,
    catalog::{self, Template},
};

pub const WIDTH: f32 = 344.0;
pub const MARGIN: f32 = 12.0;
const RADIUS: f32 = 24.0;
const PADDING: f32 = 18.0;
const PREVIEW_CELL: f32 = 60.0;
const PREVIEW_GAP: (f32, f32) = (22.5, 24.6);
const HINT: &str =
    "Drag a widget onto the desktop or click it to add it. Drag a widget back here to remove it.";

/// Whether `point` is over the open gallery.
pub fn covers(runtime: &mut Runtime, desk: Desk, point: Point) -> bool {
    let open = desk.gallery.get(runtime);
    let width = desk.surface.get(runtime).width;
    open && point.x >= width - WIDTH - MARGIN
}

fn preview_size(columns: u32, rows: u32) -> Size {
    let span =
        |count: u32, gap: f32| count as f32 * PREVIEW_CELL + count.saturating_sub(1) as f32 * gap;
    Size::new(span(columns, PREVIEW_GAP.0), span(rows, PREVIEW_GAP.1))
}

fn dim(palette: &Palette) -> Color {
    palette.panel_ink.with_alpha_factor(0.6)
}

fn card(cx: &mut Cx, desk: Desk, template: Template, palette: &Palette) -> impl View + use<> {
    let Template { title, spec } = template;
    let size = preview_size(spec.columns, spec.rows);
    let env = Env {
        ambient: desk.ambient,
        size: cx.memo(move |_| size),
        finish: Finish::Sample,
        raised: cx.signal(false),
    };
    let preview = vstack(crownhome_widgets::view(cx, &spec, env))
        .w(size.width)
        .h(size.height)
        .pointer_events_none();
    let added = spec.clone();
    carries(
        vstack((
            preview,
            text(title.clone()).text_size(12.0).text_color(dim(palette)),
        ))
        .items_center()
        .gap(8.0)
        .role(Role::Button)
        .label(title)
        .cursor(CursorIcon::Grab)
        .hover(|style| style.scale(1.03))
        .transition(Spring::snappy())
        .on_click(move |cx, event| {
            event.stop_propagation();
            desk.add(cx, added.clone());
        }),
        desk,
        Grip::Add(spec),
    )
}

fn section(
    cx: &mut Cx,
    desk: Desk,
    title: &str,
    templates: Vec<Template>,
    palette: &Palette,
) -> AnyView {
    let cards: Vec<AnyView> = templates
        .into_iter()
        .map(|template| card(cx, desk, template, palette).boxed())
        .collect();
    vstack((
        text(title.to_owned())
            .text_size(12.0)
            .font_weight(600)
            .text_color(dim(palette)),
        hstack(cards).flex_wrap().items_end().gap(16.0),
    ))
    .gap(10.0)
    .boxed()
}

fn plugins(desk: Desk, palette: Palette) -> impl View {
    let tiles = desk.ambient.plugins.tiles;
    keyed(
        move |runtime| {
            tiles.with(runtime, |tiles| {
                tiles.iter().map(catalog::of_plugin).collect::<Vec<_>>()
            })
        },
        move |cx, templates| {
            if templates.is_empty() {
                return vstack(()).display_none().boxed();
            }
            section(cx, desk, "Plugins", templates, &palette)
        },
    )
}

fn header(desk: Desk, palette: &Palette) -> impl View + use<> {
    let done = hstack(
        text("Done")
            .text_size(13.5)
            .font_weight(600)
            .text_color(WHITE),
    )
    .items_center()
    .h(28.0)
    .px(14.0)
    .rounded(14.0)
    .role(Role::Button)
    .label("Done")
    .bg(palette.accent)
    .cursor(CursorIcon::Pointer)
    .hover(|style| style.scale(1.05))
    .transition(Spring::snappy())
    .on_click(move |cx, event| {
        event.stop_propagation();
        desk.finish_editing(cx);
    });
    hstack((
        text("Add Widgets")
            .text_size(19.0)
            .font_weight(600)
            .text_color(palette.panel_ink),
        spacer(),
        done,
    ))
    .items_center()
}

fn pane(cx: &mut Cx, desk: Desk) -> impl View + use<> {
    let palette = desk.ambient.palette.get(cx);
    let sections = vstack((
        section(cx, desk, "Widgets", catalog::builtins(), &palette),
        plugins(desk, palette),
    ))
    .gap(22.0)
    .pb(PADDING);
    vstack((
        header(desk, &palette),
        text(HINT).text_size(12.5).text_color(dim(&palette)),
        scroll_view(sections).flex_1().min_h(0.0),
    ))
    .role(Role::Dialog)
    .label("Add Widgets")
    .absolute()
    .top(MARGIN)
    .bottom(MARGIN)
    .right(MARGIN)
    .w(WIDTH)
    .p(PADDING)
    .gap(12.0)
    .rounded(RADIUS)
    .border(1.0)
    .border_color(palette.panel.rim)
    .bg(palette.panel.tint)
    .material(MATERIAL)
    .enter(|style| style.translate_x(WIDTH + MARGIN).opacity(0.0))
    .exit(|style| style.translate_x(WIDTH + MARGIN).opacity(0.0))
    .transition(Spring::smooth())
    .pointer_events_auto()
    .on_click(|_, event| event.stop_propagation())
    .on_context_menu(|_, event| event.stop_propagation())
}

pub fn gallery(desk: Desk) -> impl View {
    show(
        desk.gallery,
        move || crownui::prelude::component(move |cx| pane(cx, desk)),
        || vstack(()).display_none(),
    )
    .absolute()
    .inset(0.0)
    .pointer_events_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previews_keep_the_boards_proportions() {
        let one = preview_size(1, 1);
        assert_eq!(one, Size::new(PREVIEW_CELL, PREVIEW_CELL));
        let wide = preview_size(2, 1);
        assert_eq!(wide.width, 2.0 * PREVIEW_CELL + PREVIEW_GAP.0);
    }
}
