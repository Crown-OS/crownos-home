//! The compositor opens an app's windows out of its icon, so it is told where
//! every app icon on the board rests, and asks back when one is opened.

use crownui::prelude::{Cx, Rect, Runtime, Signal, Styled, View, component, keyed, vstack};

use crate::ui::desk::Desk;
use crownhome_services::launch;
use crownhome_util::xdg;
use crownhome_widgets::app::{self, ICON_FILL, ICON_ROUNDING};

fn apps(desk: Desk, runtime: &mut Runtime) -> Vec<(app::Settings, Rect)> {
    if desk.surface.get(runtime).width <= 0.0 {
        return Vec::new();
    }
    let grid = desk.board_grid(runtime);
    desk.tiles.with(runtime, |tiles| {
        tiles
            .iter()
            .filter(|tile| tile.spec.kind == app::KIND)
            .filter_map(|tile| {
                let settings: app::Settings = tile.spec.decode().ok()?;
                let frame = grid.frame(tile.spec.area());
                let edge = frame.size.width.min(frame.size.height) * ICON_FILL;
                let center = frame.center();
                let icon =
                    Rect::from_xywh(center.x - edge / 2.0, center.y - edge / 2.0, edge, edge);
                Some((settings, icon))
            })
            .collect()
    })
}

fn report(cx: &mut Cx, reported: Signal<Vec<String>>, icons: &[(app::Settings, Rect)]) {
    let shown: Vec<String> = icons
        .iter()
        .map(|(settings, _)| settings.app_id().to_owned())
        .collect();
    let previous = reported.get(cx);
    reported.set(cx, shown.clone());
    for gone in previous.iter().filter(|app_id| !shown.contains(app_id)) {
        cx.remove_app_icon(gone);
    }
    for (settings, icon) in icons {
        cx.set_app_icon(settings.app_id(), *icon, icon.size.width * ICON_ROUNDING);
    }
}

fn launch_by_id(cx: &mut Cx, desk: Desk, app_id: &str) {
    let wanted = apps(desk, cx)
        .into_iter()
        .find(|(settings, _)| settings.app_id() == app_id)
        .and_then(|(settings, _)| xdg::find_desktop_entry(&settings.app));
    match wanted {
        Some(entry) => launch::start(entry),
        None => log::warn!("asked to start {app_id}, which is not on the board"),
    }
}

pub fn reporter(desk: Desk) -> impl View {
    component(move |cx| {
        cx.on_launch_app(move |cx, app_id| launch_by_id(cx, desk, app_id));
        let reported = cx.signal(Vec::new());
        keyed(
            move |runtime| apps(desk, runtime),
            move |cx, icons| {
                report(cx, reported, &icons);
                vstack(()).display_none()
            },
        )
    })
}
