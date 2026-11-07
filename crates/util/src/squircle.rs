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

/// The stretch of `curve` between `from` and `to`, both in `0.0..=1.0`.
pub fn segment(curve: [Point; 4], from: f32, to: f32) -> [Point; 4] {
    let (head, _) = split(curve, to);
    let (_, tail) = split(head, if to > 0.0 { from / to } else { 0.0 });
    tail
}

/// Close to the curve's arc length: halfway between its chord and its control polygon.
pub fn length([start, first, second, end]: [Point; 4]) -> f32 {
    let polygon = (first - start).length() + (second - first).length() + (end - second).length();
    (polygon + (end - start).length()) / 2.0
}

fn split([p0, p1, p2, p3]: [Point; 4], at: f32) -> ([Point; 4], [Point; 4]) {
    let lerp = |from: Point, to: Point| from + (to - from) * at;
    let (a, b, c) = (lerp(p0, p1), lerp(p1, p2), lerp(p2, p3));
    let (d, e) = (lerp(a, b), lerp(b, c));
    let middle = lerp(d, e);
    ([p0, a, d, middle], [middle, e, c, p3])
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

    #[test]
    fn a_segment_starts_and_ends_on_the_curve() {
        let curve = bottom_right(Point::new(100.0, 80.0), 20.0);
        let [start, .., end] = segment(curve, 0.0, 1.0);
        assert_eq!((start, end), (curve[0], curve[3]));
        let [half, ..] = segment(curve, 0.5, 1.0);
        let [.., also_half] = segment(curve, 0.0, 0.5);
        assert!((half - also_half).length() < 1e-4);
        assert!(length(segment(curve, 0.25, 0.75)) < length(curve));
    }
}
