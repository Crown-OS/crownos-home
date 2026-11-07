//! The widgets on the board, and the rules for where one may go.

use crate::{
    grid::{Area, BoardGrid},
    spec::WidgetSpec,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileId(pub u32);

#[derive(Debug, Clone, PartialEq)]
pub struct Tile {
    pub id: TileId,
    pub spec: WidgetSpec,
}

impl Tile {
    pub fn key(&self) -> TileId {
        self.id
    }
}

pub fn numbered(specs: Vec<WidgetSpec>) -> Vec<Tile> {
    specs
        .into_iter()
        .zip(1..)
        .map(|(spec, id)| Tile {
            id: TileId(id),
            spec,
        })
        .collect()
}

pub fn next_id(tiles: &[Tile]) -> TileId {
    TileId(tiles.iter().map(|tile| tile.id.0).max().unwrap_or(0) + 1)
}

/// Whether `area` is on the surface and clear of every tile but `moving`.
pub fn fits(tiles: &[Tile], grid: &BoardGrid, area: Area, moving: Option<TileId>) -> bool {
    area.columns > 0
        && area.rows > 0
        && grid.holds(area)
        && tiles
            .iter()
            .filter(|tile| Some(tile.id) != moving)
            .all(|tile| !tile.spec.area().overlaps(&area))
}

/// The first clear block of `columns` × `rows`, reading row by row from the
/// surface's top-left.
pub fn free_area(tiles: &[Tile], grid: &BoardGrid, columns: u32, rows: u32) -> Option<Area> {
    let (columns, rows) = (columns.max(1), rows.max(1));
    let starts = grid.columns.starts(columns);
    grid.rows
        .starts(rows)
        .flat_map(|row| starts.clone().map(move |column| (column, row)))
        .map(|(column, row)| Area {
            column,
            row,
            columns,
            rows,
        })
        .find(|area| fits(tiles, grid, *area, None))
}

/// Moves or resizes `id` to `area` when it fits there.
pub fn place(tiles: &mut [Tile], grid: &BoardGrid, id: TileId, area: Area) -> bool {
    if !fits(tiles, grid, area, Some(id)) {
        return false;
    }
    let Some(tile) = tiles.iter_mut().find(|tile| tile.id == id) else {
        return false;
    };
    if tile.spec.area() == area {
        return false;
    }
    tile.spec = tile.spec.clone().placed(area);
    true
}

#[cfg(test)]
mod tests {
    use geometry::Size;

    use super::*;
    use crate::GridSpec;

    /// A surface exactly the size of the default block, so the grid is its six by four cells.
    fn grid() -> BoardGrid {
        BoardGrid::new(GridSpec::default(), Size::new(787.5, 523.0))
    }

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

    fn board() -> Vec<Tile> {
        numbered(vec![spec(0, 0, 2, 2), spec(2, 0, 1, 1)])
    }

    #[test]
    fn a_tile_fits_where_it_already_is_but_not_over_another() {
        let grid = grid();
        let tiles = board();
        assert!(fits(&tiles, &grid, Area::cell(0, 0), Some(TileId(1))));
        assert!(!fits(&tiles, &grid, Area::cell(1, 1), Some(TileId(2))));
        assert!(!fits(&tiles, &grid, Area::cell(6, 0), None));
    }

    #[test]
    fn the_first_free_block_is_found_reading_row_by_row() {
        let grid = grid();
        let tiles = board();
        assert_eq!(free_area(&tiles, &grid, 1, 1), Some(Area::cell(3, 0)));
        let wide = free_area(&tiles, &grid, 4, 1).expect("room on the third row");
        assert_eq!((wide.column, wide.row), (2, 1));
    }

    #[test]
    fn a_full_board_has_no_free_block() {
        let spec = GridSpec {
            columns: 2,
            rows: 2,
            ..GridSpec::default()
        };
        let surface = Size::new(
            2.0 * spec.cell + spec.column_gap,
            2.0 * spec.cell + spec.row_gap,
        );
        let grid = BoardGrid::new(spec, surface);
        assert_eq!(free_area(&board(), &grid, 1, 1), None);
    }

    #[test]
    fn placing_refuses_an_overlap_and_moves_otherwise() {
        let grid = grid();
        let mut tiles = board();
        assert!(!place(&mut tiles, &grid, TileId(2), Area::cell(1, 0)));
        assert!(place(&mut tiles, &grid, TileId(2), Area::cell(5, 3)));
        assert_eq!(tiles[1].spec.area(), Area::cell(5, 3));
    }

    #[test]
    fn ids_continue_after_the_highest() {
        assert_eq!(next_id(&board()), TileId(3));
        assert_eq!(next_id(&[]), TileId(1));
    }
}
