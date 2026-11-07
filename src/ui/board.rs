//! The widgets on the board, each in its grid frame, with the handles that
//! move, resize and remove them while the board is being edited.

use crownhome_util::squircle;
use crownui::{
    ext::{DrawCommand, DrawList, Modifier, Paint, Path, Stroke, StyleProp},
    prelude::{
        Color, CursorIcon, Cx, Listeners, Memo, Point, Role, Runtime, Semantic, Signal, Size,
        Spring, Styled, View, WHITE, bind, canvas, for_each, icon, rgba, show, vstack, zstack,
    },
};

use crate::ui::{
    carry::carries,
    desk::{Desk, MenuAnchor},
    gesture::Grip,
};
use crownhome_board::{Tile, TileId};
use crownhome_widgets::{self, Env, Finish, Shape};

const BADGE: f32 = 24.0;
const BADGE_FILL: Color = rgba(0x5C5C64E6);
const BADGE_RIM: Color = rgba(0xFFFFFF66);
const OUTLINE_GAP: f32 = 5.0;
const OUTLINE_WIDTH: f32 = 1.5;
const OUTLINE_RIM: Color = rgba(0x3A3A4099);
const GRIP_WIDTH: f32 = 5.0;
const GRIP_LENGTH: f32 = 52.0;
const GRIP_INSET: f32 = GRIP_WIDTH / 2.0;
const LIFTED_SCALE: f32 = 1.04;
const MINUS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="3" stroke-linecap="round"><path d="M6 12h12"/></svg>"#;

pub fn board(desk: Desk) -> impl View {
    for_each(desk.tiles, Tile::key, move |cx, tile| {
        placed(cx, desk, tile)
    })
    .role(Role::List)
    .label("Widgets")
    .absolute()
    .inset(0.0)
    .pointer_events_none()
}

fn placed(cx: &mut Cx, desk: Desk, tile: Signal<Tile>) -> impl View + use<> {
    let (id, spec) = tile.with(cx, |tile| (tile.id, tile.spec.clone()));
    let frame = cx.memo(move |runtime| {
        let grid = desk.board_grid(runtime);
        let tile = tile.get(runtime);
        desk.gesture.with(runtime, |gesture| match gesture {
            Some(gesture) => gesture.frame_of(&tile, &grid),
            None => grid.frame(tile.spec.area()),
        })
    });
    let size = cx.memo(move |runtime| frame.get(runtime).size);
    let kind = spec.kind.clone();
    let outline = cx.memo(move |runtime| {
        crownhome_widgets::shape(&kind, size.get(runtime)).inflate(OUTLINE_GAP)
    });
    let lifted = cx.memo(move |runtime| {
        desk.gesture.with(runtime, |gesture| {
            gesture.as_ref().is_some_and(|gesture| gesture.moves(id))
        })
    });
    let env = Env {
        ambient: desk.ambient,
        size,
        finish: Finish::Live,
        raised: cx.signal(false),
    };
    let raised = env.raised;
    let content = crownhome_widgets::view(cx, &spec, env);
    zstack((
        content,
        show(
            desk.editing,
            move || handles(desk, id, outline),
            || vstack(()),
        )
        .pointer_events_none(),
    ))
    .absolute()
    .left(bind(move |runtime| frame.get(runtime).origin.x))
    .top(bind(move |runtime| frame.get(runtime).origin.y))
    .w(bind(move |runtime| frame.get(runtime).size.width))
    .h(bind(move |runtime| frame.get(runtime).size.height))
    .scale(bind(move |runtime| {
        if lifted.get(runtime) {
            LIFTED_SCALE
        } else {
            1.0
        }
    }))
    .transition(Spring::snappy())
    .modifier(Modifier::Bound(Box::new(move |runtime| {
        StyleProp::ZIndex(i16::from(raised.get(runtime)))
    })))
    .role(Role::ListItem)
    .label(spec.kind.clone())
    .pointer_events_auto()
    .on_context_menu(move |cx, event| {
        let at = desk.pointer.get(cx);
        desk.menu.set(cx, Some(MenuAnchor { at, tile: Some(id) }));
        event.stop_propagation();
    })
}

/// A round dark badge holding a white glyph, astride a corner of the widget.
fn badge(svg: &'static str, edge: f32) -> impl Listeners + View {
    vstack(icon(svg, WHITE).w(14.0).h(14.0))
        .absolute()
        .w(edge)
        .h(edge)
        .rounded(edge / 2.0)
        .items_center()
        .justify_center()
        .bg(BADGE_FILL)
        .border(1.0)
        .border_color(BADGE_RIM)
        .hover(|style| style.scale(1.1))
        .transition(Spring::snappy())
}

fn handles(desk: Desk, id: TileId, outline: Memo<Shape>) -> impl View {
    let mover = carries(
        vstack(())
            .absolute()
            .inset(0.0)
            .cursor(CursorIcon::Grab)
            .on_click(|_, event| event.stop_propagation()),
        desk,
        Grip::Move(id),
    );
    let bounds = move |runtime: &mut Runtime| outline.get(runtime).bounds;
    let remove = badge(MINUS, BADGE)
        .left(bind(move |runtime| bounds(runtime).min_x() - BADGE / 3.0))
        .top(bind(move |runtime| bounds(runtime).min_y() - BADGE / 3.0))
        .cursor(CursorIcon::Pointer)
        .role(Role::Button)
        .label("Remove Widget")
        .on_click(move |cx, event| {
            event.stop_propagation();
            desk.remove(cx, id);
        });
    let corner = carries(
        grip(outline)
            .cursor(CursorIcon::NwseResize)
            .role(Role::Button)
            .label("Resize Widget")
            .on_click(|_, event| event.stop_propagation()),
        desk,
        Grip::Resize(id),
    );
    zstack((rim(outline), mover, remove, corner))
        .absolute()
        .inset(0.0)
        .pointer_events_auto()
}

/// A thin rim just outside the widget's own outline, marking what the handles hold.
fn rim(outline: Memo<Shape>) -> impl View {
    let bounds = move |runtime: &mut Runtime| outline.get(runtime).bounds;
    vstack(())
        .absolute()
        .left(bind(move |runtime| bounds(runtime).min_x()))
        .top(bind(move |runtime| bounds(runtime).min_y()))
        .w(bind(move |runtime| bounds(runtime).size.width))
        .h(bind(move |runtime| bounds(runtime).size.height))
        .rounded(bind(move |runtime| outline.get(runtime).radius))
        .border(OUTLINE_WIDTH)
        .border_color(OUTLINE_RIM)
        .pointer_events_none()
}

/// The middle stretch of a corner of `radius`, about `GRIP_LENGTH` long.
fn grip_path(corner: Point, radius: f32) -> Path {
    let curve = squircle::bottom_right(corner, radius);
    let half_span = (GRIP_LENGTH / squircle::length(curve)).min(1.0) / 2.0;
    let [start, first, second, end] = squircle::segment(curve, 0.5 - half_span, 0.5 + half_span);
    Path::new().move_to(start).cubic_to(first, second, end)
}

fn paint_grip(list: &mut DrawList, size: Size, radius: f32) {
    let path = grip_path(
        Point::new(size.width - GRIP_INSET, size.height - GRIP_INSET),
        radius,
    );
    list.push(DrawCommand::StrokePath(
        path,
        Stroke::rounded(GRIP_WIDTH),
        Paint::solid(WHITE),
    ));
}

/// A white stroke hugging the outline's bottom-right corner, dragged to resize.
fn grip(outline: Memo<Shape>) -> impl Listeners + View {
    let radius = move |runtime: &mut Runtime| outline.get(runtime).radius;
    let edge = move |runtime: &mut Runtime| radius(runtime) * squircle::EXTENT + 2.0 * GRIP_INSET;
    let bounds = move |runtime: &mut Runtime| outline.get(runtime).bounds;
    canvas(move |runtime, size, list| paint_grip(list, size, radius(runtime)))
        .absolute()
        .left(bind(move |runtime| {
            bounds(runtime).max_x() + GRIP_INSET - edge(runtime)
        }))
        .top(bind(move |runtime| {
            bounds(runtime).max_y() + GRIP_INSET - edge(runtime)
        }))
        .w(bind(edge))
        .h(bind(edge))
}
