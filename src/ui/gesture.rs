//! Moving, resizing and adding widgets while the board is being edited.
//!
//! A gesture only records where the pointer started and how far it has
//! travelled; where a widget would land is worked out from that on demand, so
//! the board, the landing spot and the release all agree.

use crownui::prelude::{Point, Rect, Size, Vector};

use crownhome_board::{Area, BoardGrid, Tile, TileId, WidgetSpec, tile};

#[derive(Debug, Clone, PartialEq)]
pub enum Grip {
    Move(TileId),
    Resize(TileId),
    /// A widget carried in from the gallery, already sized.
    Add(WidgetSpec),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gesture {
    pub grip: Grip,
    /// The pointer when the gesture began, in surface coordinates.
    pub from: Point,
    pub travel: Vector,
}

/// Where a gesture would put its widget, and whether it may go there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Landing {
    pub area: Area,
    pub fits: bool,
}

/// What releasing a gesture does to the board.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Place(TileId, Area),
    Add(WidgetSpec),
    Remove(TileId),
    Nothing,
}

impl Gesture {
    pub fn new(grip: Grip, from: Point) -> Self {
        Self {
            grip,
            from,
            travel: Vector::new(0.0, 0.0),
        }
    }

    pub fn pointer(&self) -> Point {
        self.from + self.travel
    }

    pub fn moves(&self, id: TileId) -> bool {
        matches!(self.grip, Grip::Move(moving) | Grip::Resize(moving) if moving == id)
    }

    /// The frame `tile` is drawn in while this gesture holds it.
    pub fn frame_of(&self, tile: &Tile, grid: &BoardGrid) -> Rect {
        let rest = grid.frame(tile.spec.area());
        match self.grip {
            Grip::Move(id) if id == tile.id => rest.translate(self.travel),
            Grip::Resize(id) if id == tile.id => {
                let cell = grid.spec.cell;
                Rect::new(
                    rest.origin,
                    Size::new(
                        (rest.size.width + self.travel.x).max(cell),
                        (rest.size.height + self.travel.y).max(cell),
                    ),
                )
            }
            _ => rest,
        }
    }

    /// The frame of the widget being added, centred under the pointer.
    pub fn carried_frame(&self, grid: &BoardGrid) -> Option<Rect> {
        let Grip::Add(spec) = &self.grip else {
            return None;
        };
        let size = grid.frame(spec.area()).size;
        let pointer = self.pointer();
        Some(Rect::new(
            Point::new(pointer.x - size.width / 2.0, pointer.y - size.height / 2.0),
            size,
        ))
    }

    pub fn landing(&self, tiles: &[Tile], grid: &BoardGrid) -> Option<Landing> {
        let (area, moving) = match &self.grip {
            Grip::Move(id) => {
                let tile = tiles.iter().find(|tile| tile.id == *id)?;
                let frame = self.frame_of(tile, grid);
                let area = tile.spec.area();
                (grid.snap(frame.origin, area.columns, area.rows), Some(*id))
            }
            Grip::Resize(id) => {
                let tile = tiles.iter().find(|tile| tile.id == *id)?;
                let (columns, rows) = grid.span(self.frame_of(tile, grid).size);
                let area = tile.spec.area();
                let area = Area {
                    columns: columns.min(grid.columns.room(area.column)).max(1),
                    rows: rows.min(grid.rows.room(area.row)).max(1),
                    ..area
                };
                (area, Some(*id))
            }
            Grip::Add(spec) => {
                let frame = self.carried_frame(grid)?;
                (grid.snap(frame.origin, spec.columns, spec.rows), None)
            }
        };
        Some(Landing {
            area,
            fits: tile::fits(tiles, grid, area, moving),
        })
    }

    /// What letting go here does. `over_gallery` is whether the pointer is
    /// over the gallery, where a widget is put away.
    pub fn release(&self, tiles: &[Tile], grid: &BoardGrid, over_gallery: bool) -> Outcome {
        match (&self.grip, over_gallery) {
            (Grip::Move(id), true) => return Outcome::Remove(*id),
            (Grip::Add(_), true) => return Outcome::Nothing,
            _ => {}
        }
        let Some(landing) = self.landing(tiles, grid).filter(|landing| landing.fits) else {
            return Outcome::Nothing;
        };
        match &self.grip {
            Grip::Move(id) | Grip::Resize(id) => Outcome::Place(*id, landing.area),
            Grip::Add(spec) => Outcome::Add(spec.clone().placed(landing.area)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crownhome_board::{GridSpec, tile::numbered};

    fn spec(column: i32, row: i32, columns: u32, rows: u32) -> WidgetSpec {
        WidgetSpec {
            kind: "x".into(),
            column,
            row,
            columns,
            rows,
            settings: toml::Table::new(),
        }
    }

    fn setup() -> (Vec<Tile>, BoardGrid) {
        let tiles = numbered(vec![spec(0, 0, 2, 1), spec(2, 0, 1, 1)]);
        (
            tiles,
            BoardGrid::new(GridSpec::default(), Size::new(1000.0, 612.0)),
        )
    }

    fn pitch() -> Vector {
        let spec = GridSpec::default();
        Vector::new(spec.cell + spec.column_gap, spec.cell + spec.row_gap)
    }

    #[test]
    fn a_widget_moved_two_cells_down_lands_there() {
        let (tiles, grid) = setup();
        let mut gesture = Gesture::new(Grip::Move(TileId(1)), Point::new(0.0, 0.0));
        gesture.travel = Vector::new(10.0, pitch().y * 2.0 - 12.0);
        let area = Area {
            column: 0,
            row: 2,
            columns: 2,
            rows: 1,
        };
        assert_eq!(
            gesture.release(&tiles, &grid, false),
            Outcome::Place(TileId(1), area)
        );
    }

    #[test]
    fn a_widget_moved_onto_another_goes_nowhere() {
        let (tiles, grid) = setup();
        let mut gesture = Gesture::new(Grip::Move(TileId(2)), Point::new(0.0, 0.0));
        gesture.travel = Vector::new(-pitch().x, 0.0);
        assert_eq!(gesture.release(&tiles, &grid, false), Outcome::Nothing);
        assert_eq!(
            gesture.landing(&tiles, &grid).map(|landing| landing.fits),
            Some(false)
        );
    }

    #[test]
    fn a_widget_dragged_onto_the_gallery_is_removed() {
        let (tiles, grid) = setup();
        let gesture = Gesture::new(Grip::Move(TileId(2)), Point::new(0.0, 0.0));
        assert_eq!(
            gesture.release(&tiles, &grid, true),
            Outcome::Remove(TileId(2))
        );
    }

    #[test]
    fn stretching_a_corner_by_a_cell_grows_the_widget_by_one() {
        let (tiles, grid) = setup();
        let mut gesture = Gesture::new(Grip::Resize(TileId(2)), Point::new(0.0, 0.0));
        gesture.travel = Vector::new(pitch().x * 0.6, pitch().y * 1.2);
        let grown = Area {
            column: 2,
            row: 0,
            columns: 2,
            rows: 2,
        };
        assert_eq!(
            gesture.release(&tiles, &grid, false),
            Outcome::Place(TileId(2), grown)
        );
    }

    #[test]
    fn a_widget_cannot_shrink_below_one_cell() {
        let (tiles, grid) = setup();
        let mut gesture = Gesture::new(Grip::Resize(TileId(1)), Point::new(0.0, 0.0));
        gesture.travel = Vector::new(-900.0, -900.0);
        let landing = gesture
            .landing(&tiles, &grid)
            .expect("the tile is on the board");
        assert_eq!((landing.area.columns, landing.area.rows), (1, 1));
    }

    #[test]
    fn a_widget_carried_from_the_gallery_lands_under_the_pointer() {
        let (tiles, grid) = setup();
        let target = grid.frame(Area::cell(4, 2)).center();
        let gesture = Gesture::new(Grip::Add(spec(0, 0, 1, 1)), target);
        assert_eq!(
            gesture.release(&tiles, &grid, false),
            Outcome::Add(spec(4, 2, 1, 1))
        );
        assert_eq!(gesture.release(&tiles, &grid, true), Outcome::Nothing);
    }
}
