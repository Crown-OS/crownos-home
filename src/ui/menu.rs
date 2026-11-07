//! The desktop's context menu: frosted glass, compact rows, a pill-shaped
//! highlight the panel's corners stay concentric with, and hairline
//! separators.

use crownui::prelude::{
    AnyView, Cx, Listeners, Point, Role, Semantic, Size, Spring, Styled, TRANSPARENT, View, hstack,
    keyed, text, vstack,
};

use crate::ui::desk::{Desk, MenuAnchor};
use crownhome_board::TileId;
use crownhome_services::launch;
use crownhome_widgets::palette::{MATERIAL, Palette};

const WIDTH: f32 = 248.0;
const PADDING: f32 = 6.0;
const RIM: f32 = 1.0;
const ROW_HEIGHT: f32 = 28.0;
/// The highlight is a pill, and the panel's corners keep one gap from it all
/// the way round, so their radius is the pill's plus that gap.
const ROW_RADIUS: f32 = ROW_HEIGHT / 2.0;
const RADIUS: f32 = ROW_RADIUS + PADDING + RIM;
const TEXT_INSET: f32 = PADDING + ROW_RADIUS;
const TEXT_SIZE: f32 = 15.0;
const SEPARATOR_MARGIN: f32 = 6.0;
const APPEAR_SCALE: f32 = 0.97;
/// The compositor blurs only what lies under the desktop; widgets the menu
/// opens over are this surface's own, so the menu blurs them itself.
const BACKDROP_BLUR: f32 = 32.0;
const SETTINGS: [&str; 2] = ["crownsettings", "appearance"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    AddWidgets,
    EditWidgets,
    DoneEditing,
    RemoveWidget(TileId),
    ChangeWallpaper,
    ResetWidgets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    Item(&'static str, Command),
    Separator,
}

fn entries(editing: bool, tile: Option<TileId>) -> Vec<Entry> {
    let edit = if editing {
        Entry::Item("Done Editing", Command::DoneEditing)
    } else {
        Entry::Item("Edit Widgets", Command::EditWidgets)
    };
    let add = Entry::Item("Add Widgets…", Command::AddWidgets);
    let wallpaper = Entry::Item("Change Wallpaper…", Command::ChangeWallpaper);
    match tile {
        Some(id) => vec![
            Entry::Item("Remove Widget", Command::RemoveWidget(id)),
            Entry::Separator,
            edit,
            add,
            Entry::Separator,
            wallpaper,
        ],
        None => vec![
            add,
            edit,
            Entry::Separator,
            wallpaper,
            Entry::Separator,
            Entry::Item("Reset Widgets", Command::ResetWidgets),
        ],
    }
}

fn height(entries: &[Entry]) -> f32 {
    let rows: f32 = entries
        .iter()
        .map(|entry| match entry {
            Entry::Item(..) => ROW_HEIGHT,
            Entry::Separator => 1.0 + 2.0 * SEPARATOR_MARGIN,
        })
        .sum();
    rows + 2.0 * (PADDING + RIM)
}

/// Where a menu opened at `point` goes: right of and below it, flipped where
/// it would cross the surface's edge.
fn place(point: Point, panel: Size, surface: Size) -> Point {
    let axis = |start: f32, extent: f32, limit: f32| {
        if limit <= 0.0 || start + extent <= limit {
            start
        } else if start - extent >= 0.0 {
            start - extent
        } else {
            (limit - extent).max(0.0)
        }
    };
    Point::new(
        axis(point.x, panel.width, surface.width),
        axis(point.y, panel.height, surface.height),
    )
}

fn perform(cx: &mut Cx, desk: Desk, command: Command) {
    desk.menu.set(cx, None);
    match command {
        Command::AddWidgets => desk.start_editing(cx, true),
        Command::EditWidgets => desk.start_editing(cx, false),
        Command::DoneEditing => desk.finish_editing(cx),
        Command::RemoveWidget(id) => desk.remove(cx, id),
        Command::ChangeWallpaper => launch::run(SETTINGS.map(str::to_owned).to_vec()),
        Command::ResetWidgets => desk.reset(cx),
    }
}

fn row(desk: Desk, entry: Entry, palette: &Palette) -> AnyView {
    match entry {
        Entry::Separator => vstack(())
            .h(1.0)
            .mx(TEXT_INSET)
            .my(SEPARATOR_MARGIN)
            .bg(palette.separator)
            .boxed(),
        Entry::Item(label, command) => {
            let highlight = palette.highlight();
            hstack(
                text(label)
                    .text_size(TEXT_SIZE)
                    .text_color(palette.panel_ink),
            )
            .items_center()
            .h(ROW_HEIGHT)
            .px(TEXT_INSET - PADDING)
            .rounded(ROW_RADIUS)
            .role(Role::MenuItem)
            .label(label)
            .bg(TRANSPARENT)
            .hover(move |style| style.bg(highlight))
            .on_click(move |cx, event| {
                event.stop_propagation();
                perform(cx, desk, command);
            })
            .boxed()
        }
    }
}

fn panel(cx: &mut Cx, desk: Desk, anchor: MenuAnchor) -> AnyView {
    let palette = desk.ambient.palette.get(cx);
    let editing = desk.editing.get(cx);
    let entries = entries(editing, anchor.tile);
    let size = Size::new(WIDTH, height(&entries));
    let at = place(anchor.at, size, desk.surface.get(cx));
    let rows: Vec<AnyView> = entries
        .into_iter()
        .map(|entry| row(desk, entry, &palette))
        .collect();
    let panel = vstack(rows)
        .role(Role::Menu)
        .absolute()
        .left(at.x)
        .top(at.y)
        .w(size.width)
        .p(PADDING)
        .rounded(RADIUS)
        .border(RIM)
        .border_color(palette.panel.rim)
        .bg(palette.panel.tint)
        .backdrop_blur(BACKDROP_BLUR)
        .material(MATERIAL)
        .origin(0.0, 0.0)
        .enter(|style| style.opacity(0.0).scale(APPEAR_SCALE))
        .transition(Spring::crisp())
        .on_pointer_down(|_, event| event.stop_propagation())
        .on_context_menu(|_, event| event.stop_propagation());
    vstack(panel)
        .absolute()
        .inset(0.0)
        .pointer_events_auto()
        .on_pointer_down(move |cx, _| desk.menu.set(cx, None))
        .boxed()
}

pub fn menu(desk: Desk) -> impl View {
    keyed(
        move |runtime| desk.menu.get(runtime),
        move |cx, anchor| match anchor {
            Some(anchor) => panel(cx, desk, anchor),
            None => vstack(()).display_none().boxed(),
        },
    )
    .absolute()
    .inset(0.0)
    .pointer_events_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_desktop_menu_opens_with_add_widgets() {
        let first = entries(false, None)[0];
        assert_eq!(first, Entry::Item("Add Widgets…", Command::AddWidgets));
        assert!(entries(true, None).contains(&Entry::Item("Done Editing", Command::DoneEditing)));
    }

    #[test]
    fn a_widgets_menu_can_remove_it() {
        let first = entries(false, Some(TileId(4)))[0];
        assert_eq!(
            first,
            Entry::Item("Remove Widget", Command::RemoveWidget(TileId(4)))
        );
    }

    #[test]
    fn rows_and_separators_add_up_to_the_panel_height() {
        let shown = [Entry::Item("a", Command::AddWidgets), Entry::Separator];
        assert_eq!(height(&shown), ROW_HEIGHT + 13.0 + 14.0);
    }

    #[test]
    fn a_menu_near_the_corner_flips_up_and_left() {
        let surface = Size::new(800.0, 600.0);
        let panel = Size::new(248.0, 150.0);
        assert_eq!(
            place(Point::new(100.0, 80.0), panel, surface),
            Point::new(100.0, 80.0)
        );
        assert_eq!(
            place(Point::new(700.0, 500.0), panel, surface),
            Point::new(452.0, 350.0)
        );
    }
}
