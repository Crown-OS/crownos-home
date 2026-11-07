//! The desktop's state, and the one place the board changes.

use std::path::PathBuf;

use crownui::prelude::{Cx, Point, Runtime, Signal, Size};

use crate::ui::gesture::{Gesture, Outcome};
use crownhome_board::{
    Area, BoardGrid, BoardSpec, GridSpec, Tile, TileId, WidgetSpec,
    tile::{self, numbered},
};
use crownhome_widgets::Ambient;

/// Where a context menu was opened, and on which widget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuAnchor {
    pub at: Point,
    pub tile: Option<TileId>,
}

#[derive(Clone, Copy)]
pub struct Desk {
    pub grid: GridSpec,
    pub tiles: Signal<Vec<Tile>>,
    pub surface: Signal<Size>,
    /// Widgets show their handles and can be moved, resized and removed.
    pub editing: Signal<bool>,
    /// The widget gallery is open along the right edge.
    pub gallery: Signal<bool>,
    pub menu: Signal<Option<MenuAnchor>>,
    pub gesture: Signal<Option<Gesture>>,
    /// The pointer at the last press, in surface coordinates.
    pub pointer: Signal<Point>,
    file: Signal<Option<PathBuf>>,
    pub ambient: Ambient,
}

impl Desk {
    pub fn new(cx: &mut Cx, spec: BoardSpec, file: Option<PathBuf>, ambient: Ambient) -> Self {
        Self {
            grid: spec.grid,
            tiles: cx.signal(numbered(spec.widgets)),
            surface: cx.signal(Size::new(0.0, 0.0)),
            editing: cx.signal(false),
            gallery: cx.signal(false),
            menu: cx.signal(None),
            gesture: cx.signal(None),
            pointer: cx.signal(Point::new(0.0, 0.0)),
            file: cx.signal(file),
            ambient,
        }
    }

    pub fn board_grid(self, runtime: &mut Runtime) -> BoardGrid {
        BoardGrid::new(self.grid, self.surface.get(runtime))
    }

    /// Applies `edit` to the tiles and saves the board when it reports a change.
    fn change(self, cx: &mut Cx, edit: impl FnOnce(&mut Vec<Tile>, &BoardGrid) -> bool) {
        let grid = self.board_grid(cx);
        let mut tiles = self.tiles.get(cx);
        if edit(&mut tiles, &grid) {
            self.tiles.set(cx, tiles);
            self.save(cx);
        }
    }

    /// Adds `spec` where it says, or in the first space big enough for it.
    pub fn add(self, cx: &mut Cx, spec: WidgetSpec) {
        self.change(cx, |tiles, grid| {
            let wanted = spec.area();
            let area = if tile::fits(tiles, grid, wanted, None) {
                Some(wanted)
            } else {
                tile::free_area(tiles, grid, wanted.columns, wanted.rows)
            };
            let Some(area) = area else {
                log::info!("no room on the board for a {} widget", spec.kind);
                return false;
            };
            let id = tile::next_id(tiles);
            tiles.push(Tile {
                id,
                spec: spec.placed(area),
            });
            true
        });
    }

    pub fn remove(self, cx: &mut Cx, id: TileId) {
        self.change(cx, |tiles, _| {
            let before = tiles.len();
            tiles.retain(|tile| tile.id != id);
            tiles.len() != before
        });
    }

    pub fn place(self, cx: &mut Cx, id: TileId, area: Area) {
        self.change(cx, |tiles, grid| tile::place(tiles, grid, id, area));
    }

    /// Puts the built-in board back.
    pub fn reset(self, cx: &mut Cx) {
        let defaults = BoardSpec::default();
        self.change(cx, |tiles, _| {
            *tiles = numbered(defaults.widgets);
            true
        });
    }

    /// Lifts `id` above the others while it is carried, by moving it last.
    pub fn raise(self, cx: &mut Cx, id: TileId) {
        self.tiles.update(cx, |tiles| {
            if let Some(index) = tiles.iter().position(|tile| tile.id == id) {
                let lifted = tiles.remove(index);
                tiles.push(lifted);
            }
        });
    }

    pub fn apply(self, cx: &mut Cx, outcome: Outcome) {
        match outcome {
            Outcome::Place(id, area) => self.place(cx, id, area),
            Outcome::Add(spec) => self.add(cx, spec),
            Outcome::Remove(id) => self.remove(cx, id),
            Outcome::Nothing => {}
        }
    }

    pub fn start_editing(self, cx: &mut Cx, with_gallery: bool) {
        self.menu.set(cx, None);
        self.editing.set(cx, true);
        if with_gallery {
            self.gallery.set(cx, true);
        }
    }

    pub fn finish_editing(self, cx: &mut Cx) {
        self.gesture.set(cx, None);
        self.gallery.set(cx, false);
        self.editing.set(cx, false);
    }

    fn save(self, cx: &mut Cx) {
        let spec = BoardSpec {
            grid: self.grid,
            widgets: self.tiles.with(cx, |tiles| {
                tiles.iter().map(|tile| tile.spec.clone()).collect()
            }),
        };
        let saved = self
            .file
            .with(cx, |file| file.as_deref().map(|file| spec.save(file)));
        if let Some(Err(e)) = saved {
            log::warn!("saving the widget board: {e}");
        }
    }
}
