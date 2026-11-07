//! The desktop: the board over the wallpaper, and everything that edits it.

use std::path::PathBuf;

use crownui::{
    ext::{DragOverEvent, DropEvent},
    kit::{INTER_FAMILY, ThemeMode, provide_configured_theme},
    prelude::{
        Color, Cx, Listeners, Rect, Role, Runtime, Semantic, Spring, Styled, View, bind, component,
        keyed, rgba, vstack, zstack,
    },
};

use crate::ui::{
    app_icons,
    board::board,
    desk::{Desk, MenuAnchor},
    gallery::{self, gallery},
    gesture::Grip,
    menu::menu,
    probe::size_probe,
};
use crownhome_board::{BoardSpec, WidgetSpec};
use crownhome_services::{clock, plugins::Plugins};
use crownhome_util::uri::{URI_LIST, parse_uri_list};
use crownhome_widgets::palette::Palette;
use crownhome_widgets::{self, Ambient, Env, Finish, app};

const LANDING_RADIUS: f32 = 24.0;
const LANDING_FITS: (Color, Color) = (rgba(0xFFFFFF29), rgba(0xFFFFFFB3));
const LANDING_BLOCKED: (Color, Color) = (rgba(0xFF3B3029), rgba(0xFF6B60B3));
const CARRIED_OPACITY: f32 = 0.9;

fn landing_frame(desk: Desk, runtime: &mut Runtime) -> Option<(Rect, bool)> {
    let gesture = desk.gesture.get(runtime)?;
    if gallery::covers(runtime, desk, gesture.pointer()) {
        return None;
    }
    let grid = desk.board_grid(runtime);
    let landing = desk
        .tiles
        .with(runtime, |tiles| gesture.landing(tiles, &grid))?;
    Some((grid.frame(landing.area), landing.fits))
}

/// The cells a carried widget would land on, clear when it fits there and red
/// when it does not.
fn landing(cx: &mut Cx, desk: Desk) -> impl View + use<> {
    let shown = cx.memo(move |runtime| landing_frame(desk, runtime));
    let frame = move |runtime: &mut Runtime| {
        shown
            .get(runtime)
            .map_or(Rect::default(), |(frame, _)| frame)
    };
    let colors = move |runtime: &mut Runtime| {
        if shown.get(runtime).is_none_or(|(_, fits)| fits) {
            LANDING_FITS
        } else {
            LANDING_BLOCKED
        }
    };
    vstack(())
        .absolute()
        .left(bind(move |runtime| frame(runtime).origin.x))
        .top(bind(move |runtime| frame(runtime).origin.y))
        .w(bind(move |runtime| frame(runtime).size.width))
        .h(bind(move |runtime| frame(runtime).size.height))
        .rounded(LANDING_RADIUS)
        .bg(bind(move |runtime| colors(runtime).0))
        .border(1.5)
        .border_color(bind(move |runtime| colors(runtime).1))
        .hidden(bind(move |runtime| shown.get(runtime).is_none()))
        .transition(Spring::snappy())
        .pointer_events_none()
}

/// The widget being carried in from the gallery, under the pointer.
fn carried(desk: Desk) -> impl View {
    keyed(
        move |runtime| {
            desk.gesture.with(runtime, |gesture| {
                match gesture.as_ref().map(|gesture| &gesture.grip) {
                    Some(Grip::Add(spec)) => Some(spec.clone()),
                    _ => None,
                }
            })
        },
        move |cx, spec: Option<WidgetSpec>| {
            let Some(spec) = spec else {
                return vstack(()).display_none().boxed();
            };
            let frame = cx.memo(move |runtime| {
                let grid = desk.board_grid(runtime);
                desk.gesture
                    .with(runtime, |gesture| gesture.as_ref()?.carried_frame(&grid))
                    .unwrap_or_default()
            });
            let env = Env {
                ambient: desk.ambient,
                size: cx.memo(move |runtime| frame.get(runtime).size),
                finish: Finish::Sample,
            };
            vstack(crownhome_widgets::view(cx, &spec, env))
                .absolute()
                .left(bind(move |runtime| frame.get(runtime).origin.x))
                .top(bind(move |runtime| frame.get(runtime).origin.y))
                .w(bind(move |runtime| frame.get(runtime).size.width))
                .h(bind(move |runtime| frame.get(runtime).size.height))
                .opacity(CARRIED_OPACITY)
                .pointer_events_none()
                .boxed()
        },
    )
    .absolute()
    .inset(0.0)
    .pointer_events_none()
}

fn drag_over(event: &mut DragOverEvent) {
    if event.offers(URI_LIST) {
        event.accept(URI_LIST);
    }
}

/// App entries dropped from the dock or the launcher become app widgets
/// where they land.
fn dropped(cx: &mut Cx, desk: Desk, event: &DropEvent) {
    let grid = desk.board_grid(cx);
    let entries = parse_uri_list(&event.data).filter(|path| {
        path.extension()
            .is_some_and(|extension| extension == "desktop")
    });
    for entry in entries {
        let area = grid.snap(event.position, 1, 1);
        let settings = toml::Value::try_from(app::Settings::of_entry(&entry));
        let Ok(toml::Value::Table(settings)) = settings else {
            continue;
        };
        let spec = WidgetSpec {
            kind: app::KIND.to_owned(),
            column: area.column,
            row: area.row,
            columns: 1,
            rows: 1,
            settings,
        };
        desk.add(cx, spec);
    }
}

fn press_elsewhere(cx: &mut Cx, desk: Desk) {
    if desk.menu.get(cx).is_some() {
        desk.menu.set(cx, None);
    } else if desk.editing.get(cx) {
        desk.finish_editing(cx);
    }
}

pub fn root(spec: BoardSpec, file: Option<PathBuf>) -> impl View {
    component(move |cx| {
        let theme = provide_configured_theme(cx);
        let capabilities = cx.platform_capabilities();
        let palette = cx.memo(move |runtime| {
            let (dark, accent) = theme.with(runtime, |theme| {
                (matches!(theme.mode, ThemeMode::Dark), theme.accent.start)
            });
            let blurred = capabilities.with(runtime, |capabilities| capabilities.materials);
            Palette::new(dark, accent, blurred)
        });
        let ambient = Ambient {
            now: clock::start(cx),
            palette,
            plugins: Plugins::start(cx),
        };
        let desk = Desk::new(cx, spec, file, ambient);
        zstack((
            size_probe(desk.surface).absolute().inset(0.0),
            app_icons::reporter(desk),
            board(desk),
            landing(cx, desk),
            carried(desk),
            gallery(desk),
            menu(desk),
        ))
        .relative()
        .w_full()
        .h_full()
        .role(Role::Window)
        .label("Desktop")
        .font_family(INTER_FAMILY)
        .on_pointer_down(move |cx, event| desk.pointer.set(cx, event.position))
        .on_context_menu(move |cx, _| {
            let at = desk.pointer.get(cx);
            desk.menu.set(cx, Some(MenuAnchor { at, tile: None }));
        })
        .on_click(move |cx, _| press_elsewhere(cx, desk))
        .on_drag_over(|_, event| drag_over(event))
        .on_drop(move |cx, event| dropped(cx, desk, event))
    })
}
