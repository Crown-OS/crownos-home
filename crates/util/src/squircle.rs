//! The continuous corner CrownOS draws, as a cubic, for strokes that follow a
//! widget's outline.

use geometry::{Point, Vector};

/// How far along each edge a corner reaches, as a multiple of its radius; the
/// compositor's and Impeller's `EXTENT`.
pub const EXTENT: f32 = 1.5;
const HANDLE: f32 = 0.919_21;

/// The bottom-right corner at `corner` with `radius`: where it leaves the right
/// edge, its two handles, and where it joins the bottom edge.
pub fn bottom_right(corner: Point, radius: f32) -> [Point; 4] {
    let reach = radius * EXTENT;
    let slack = reach * (1.0 - HANDLE);
    let up = Vector::new(0.0, 1.0);
    let left = Vector::new(1.0, 0.0);
    [
        corner - up * reach,
        corner - up * slack,
        corner - left * slack,
        corner - left * reach,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_corner_leaves_and_joins_its_edges_a_reach_away() {
        let [start, .., end] = bottom_right(Point::new(100.0, 80.0), 20.0);
        assert_eq!(start, Point::new(100.0, 50.0));
        assert_eq!(end, Point::new(70.0, 80.0));
    }
}
