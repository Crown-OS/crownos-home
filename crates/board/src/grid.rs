//! The grid widgets snap to, spread over the whole surface.

use geometry::{Point, Rect, Size};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GridSpec {
    /// Edge of one square cell, in logical pixels.
    pub cell: f32,
    pub column_gap: f32,
    pub row_gap: f32,
    /// The block centred on the surface that the cells line up with; cell
    /// `(0, 0)` is its top-left one.
    pub columns: u32,
    pub rows: u32,
}

impl Default for GridSpec {
    fn default() -> Self {
        Self {
            cell: 100.0,
            column_gap: 37.5,
            row_gap: 41.0,
            columns: 6,
            rows: 4,
        }
    }
}

/// A block of cells, top-left first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Area {
    pub column: i32,
    pub row: i32,
    pub columns: u32,
    pub rows: u32,
}

impl Area {
    pub const fn cell(column: i32, row: i32) -> Self {
        Self {
            column,
            row,
            columns: 1,
            rows: 1,
        }
    }

    fn end_column(&self) -> i32 {
        self.column + self.columns as i32
    }

    fn end_row(&self) -> i32 {
        self.row + self.rows as i32
    }

    pub fn overlaps(&self, other: &Area) -> bool {
        self.column < other.end_column()
            && other.column < self.end_column()
            && self.row < other.end_row()
            && other.row < self.end_row()
    }
}

/// The cells along one axis that fit on the surface: the first one's index and how many.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lane {
    pub first: i32,
    pub count: u32,
}

impl Lane {
    fn new(origin: f32, pitch: f32, cell: f32, length: f32) -> Self {
        let first = (-origin / pitch).ceil() as i32;
        let last = ((length - cell - origin) / pitch).floor() as i32;
        Self {
            first,
            count: u32::try_from(last - first + 1).unwrap_or(0),
        }
    }

    fn end(self) -> i32 {
        self.first + self.count as i32
    }

    /// How many cells there are from `start` to the far edge.
    pub fn room(self, start: i32) -> u32 {
        u32::try_from(self.end() - start).unwrap_or(0)
    }

    fn holds(self, start: i32, span: u32) -> bool {
        start >= self.first && span <= self.room(start)
    }

    /// The start nearest `index` that keeps `span` cells on the lane.
    fn clamp(self, index: i32, span: u32) -> i32 {
        index.min(self.end() - span as i32).max(self.first)
    }

    /// Every start that keeps `span` cells on the lane, nearest the surface's origin first.
    pub fn starts(self, span: u32) -> std::ops::RangeInclusive<i32> {
        self.first..=self.end() - span as i32
    }
}

/// The grid laid over the whole surface. The spec's block of cells sits centred,
/// and its rhythm repeats past it to every edge, so a cell left of or above that
/// block has a negative index.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardGrid {
    pub spec: GridSpec,
    origin: Point,
    pub columns: Lane,
    pub rows: Lane,
}

impl BoardGrid {
    pub fn new(spec: GridSpec, surface: Size) -> Self {
        let extent = Self::extent(&spec);
        let origin = Point::new(
            ((surface.width - extent.width) / 2.0).round(),
            ((surface.height - extent.height) / 2.0).round(),
        );
        let (pitch_x, pitch_y) = Self::pitch_of(&spec);
        Self {
            spec,
            origin,
            columns: Lane::new(origin.x, pitch_x, spec.cell, surface.width),
            rows: Lane::new(origin.y, pitch_y, spec.cell, surface.height),
        }
    }

    fn extent(spec: &GridSpec) -> Size {
        let span = |count: u32, gap: f32| {
            let count = count.max(1) as f32;
            count * spec.cell + (count - 1.0) * gap
        };
        Size::new(
            span(spec.columns, spec.column_gap),
            span(spec.rows, spec.row_gap),
        )
    }

    fn pitch_of(spec: &GridSpec) -> (f32, f32) {
        (spec.cell + spec.column_gap, spec.cell + spec.row_gap)
    }

    fn pitch(&self) -> (f32, f32) {
        Self::pitch_of(&self.spec)
    }

    pub fn holds(&self, area: Area) -> bool {
        self.columns.holds(area.column, area.columns) && self.rows.holds(area.row, area.rows)
    }

    pub fn frame(&self, area: Area) -> Rect {
        let (pitch_x, pitch_y) = self.pitch();
        Rect::from_xywh(
            self.origin.x + area.column as f32 * pitch_x,
            self.origin.y + area.row as f32 * pitch_y,
            area.columns.max(1) as f32 * pitch_x - self.spec.column_gap,
            area.rows.max(1) as f32 * pitch_y - self.spec.row_gap,
        )
    }

    /// The block of `columns` × `rows` whose top-left cell is nearest `corner`,
    /// kept on the surface.
    pub fn snap(&self, corner: Point, columns: u32, rows: u32) -> Area {
        let (pitch_x, pitch_y) = self.pitch();
        let nearest = |offset: f32, pitch: f32| (offset / pitch).round() as i32;
        let columns = columns.clamp(1, self.columns.count.max(1));
        let rows = rows.clamp(1, self.rows.count.max(1));
        Area {
            column: self
                .columns
                .clamp(nearest(corner.x - self.origin.x, pitch_x), columns),
            row: self
                .rows
                .clamp(nearest(corner.y - self.origin.y, pitch_y), rows),
            columns,
            rows,
        }
    }

    /// How many cells a frame `size` points wide and tall covers, at least one.
    pub fn span(&self, size: Size) -> (u32, u32) {
        let (pitch_x, pitch_y) = self.pitch();
        let count =
            |length: f32, gap: f32, pitch: f32| ((length + gap) / pitch).round().max(1.0) as u32;
        (
            count(size.width, self.spec.column_gap, pitch_x).min(self.columns.count.max(1)),
            count(size.height, self.spec.row_gap, pitch_y).min(self.rows.count.max(1)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> BoardGrid {
        BoardGrid::new(GridSpec::default(), Size::new(1000.0, 612.0))
    }

    #[test]
    fn the_spec_block_is_centred_on_the_surface() {
        let spec = GridSpec::default();
        let top_left = grid().frame(Area::cell(0, 0));
        let bottom_right = grid().frame(Area::cell(spec.columns as i32 - 1, spec.rows as i32 - 1));
        assert!(((top_left.min_x() + bottom_right.max_x()) / 2.0 - 500.0).abs() <= 0.5);
        assert!(((top_left.min_y() + bottom_right.max_y()) / 2.0 - 306.0).abs() <= 0.5);
    }

    #[test]
    fn the_cells_reach_every_edge_of_a_wide_surface() {
        let grid = BoardGrid::new(GridSpec::default(), Size::new(1600.0, 1000.0));
        assert_eq!(
            grid.columns,
            Lane {
                first: -2,
                count: 10
            }
        );
        assert_eq!(
            grid.rows,
            Lane {
                first: -1,
                count: 6
            }
        );
        let left = grid.frame(Area::cell(-2, -1));
        assert!(left.min_x() >= 0.0 && left.min_y() >= 0.0);
        let right = grid.frame(Area::cell(7, 4));
        assert!(right.max_x() <= 1600.0 && right.max_y() <= 1000.0);
        assert!(!grid.holds(Area::cell(8, 0)));
        assert_eq!(
            grid.snap(Point::new(-500.0, -500.0), 1, 1),
            Area::cell(-2, -1)
        );
    }

    #[test]
    fn a_two_by_two_block_spans_one_gap_each_way() {
        let spec = GridSpec::default();
        let frame = grid().frame(Area {
            column: 0,
            row: 1,
            columns: 2,
            rows: 2,
        });
        assert_eq!(frame.size.width, 2.0 * spec.cell + spec.column_gap);
        assert_eq!(frame.size.height, 2.0 * spec.cell + spec.row_gap);
    }

    #[test]
    fn a_corner_snaps_to_the_nearest_cell_and_stays_on_the_board() {
        let grid = grid();
        let cell = grid.frame(Area::cell(2, 1));
        let nudged = Point::new(cell.min_x() + 30.0, cell.min_y() - 30.0);
        assert_eq!(grid.snap(nudged, 1, 1), Area::cell(2, 1));
        let far = Point::new(5000.0, 5000.0);
        let wide = grid.snap(far, 4, 2);
        assert_eq!((wide.column, wide.row), (2, 2));
    }

    #[test]
    fn a_frame_size_reads_back_as_its_span() {
        let grid = grid();
        let frame = grid.frame(Area {
            column: 0,
            row: 0,
            columns: 3,
            rows: 2,
        });
        assert_eq!(grid.span(frame.size), (3, 2));
        assert_eq!(grid.span(Size::new(10.0, 10.0)), (1, 1));
    }

    #[test]
    fn areas_overlap_only_when_they_share_a_cell() {
        let wide = Area {
            column: 0,
            row: 0,
            columns: 4,
            rows: 1,
        };
        assert!(wide.overlaps(&Area::cell(3, 0)));
        assert!(!wide.overlaps(&Area::cell(4, 0)));
        assert!(!wide.overlaps(&Area::cell(0, 1)));
    }
}
