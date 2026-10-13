//! Paths as polylines, for painters that draw straight segments and for
//! measuring along a path.

use crate::Seg;
use crate::geom::Point;

/// One subpath as points.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Polyline {
    pub points: Vec<Point>,
    /// Ended with [`Seg::Close`]: the last point joins the first.
    pub closed: bool,
}

/// `segs` as polylines, curves split until no point is further than
/// `tolerance` from the true curve (roughly).
pub fn flatten(segs: &[Seg], tolerance: f32) -> Vec<Polyline> {
    let mut out: Vec<Polyline> = Vec::new();
    let mut current = Polyline::default();
    let mut at = Point::default();
    let mut start = Point::default();
    for seg in segs {
        match *seg {
            Seg::Move(p) => {
                if current.points.len() > 1 {
                    out.push(std::mem::take(&mut current));
                } else {
                    current = Polyline::default();
                }
                current.points.push(p);
                at = p;
                start = p;
            }
            Seg::Line(p) => {
                if current.points.is_empty() {
                    current.points.push(at);
                }
                current.points.push(p);
                at = p;
            }
            Seg::Cubic(c1, c2, p) => {
                if current.points.is_empty() {
                    current.points.push(at);
                }
                // Enough steps that each chord is short relative to the
                // tolerance; the control polygon's length bounds the curve.
                let len = at.distance(c1) + c1.distance(c2) + c2.distance(p);
                let steps = ((len / (tolerance.max(0.05) * 8.0)).sqrt() * 4.0)
                    .ceil()
                    .clamp(2.0, 64.0) as usize;
                for i in 1..=steps {
                    current
                        .points
                        .push(cubic(at, c1, c2, p, i as f32 / steps as f32));
                }
                at = p;
            }
            Seg::Close => {
                if current.points.len() > 1 {
                    if current.points.last() == Some(&start) {
                        current.points.pop();
                    }
                    current.closed = true;
                    out.push(std::mem::take(&mut current));
                }
                at = start;
            }
        }
    }
    if current.points.len() > 1 {
        out.push(current);
    }
    out
}

/// The point at `t` on a cubic Bézier.
pub fn cubic(p0: Point, p1: Point, p2: Point, p3: Point, t: f32) -> Point {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    Point::new(
        a * p0.x + b * p1.x + c * p2.x + d * p3.x,
        a * p0.y + b * p1.y + c * p2.y + d * p3.y,
    )
}

/// The open path's points end to end (all subpaths joined).
fn points(segs: &[Seg]) -> Vec<Point> {
    flatten(segs, 0.5)
        .into_iter()
        .flat_map(|p| p.points)
        .collect()
}

/// The length of a path.
pub fn length(segs: &[Seg]) -> f32 {
    points(segs).windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// The point a fraction `t` (0 to 1) of the way along a path.
pub fn point_at(segs: &[Seg], t: f32) -> Option<Point> {
    let pts = points(segs);
    let first = *pts.first()?;
    let total: f32 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
    let mut left = total * t.clamp(0.0, 1.0);
    for w in pts.windows(2) {
        let d = w[0].distance(w[1]);
        if d >= left && d > 0.0 {
            return Some(w[0].lerp(w[1], left / d));
        }
        left -= d;
    }
    Some(*pts.last().unwrap_or(&first))
}

/// The path's last point and the unit direction it arrives in.
pub fn end(segs: &[Seg]) -> Option<(Point, Point)> {
    let mut prev: Option<Point> = None;
    let mut last: Option<Point> = None;
    for seg in segs {
        match *seg {
            Seg::Move(p) | Seg::Line(p) => {
                prev = last.or(Some(p));
                last = Some(p);
            }
            Seg::Cubic(_, c2, p) => {
                // The tangent at the end runs from the second control
                // point, unless it sits on the end.
                prev = if c2 == p { last } else { Some(c2) };
                last = Some(p);
            }
            Seg::Close => {}
        }
    }
    let (a, b) = (prev?, last?);
    Some((b, a.direction_to(b)))
}

/// The path's first point and the unit direction leaving it, reversed:
/// pointing into the start, as a marker there needs.
pub fn start(segs: &[Seg]) -> Option<(Point, Point)> {
    let mut iter = segs.iter();
    let first = match iter.next()? {
        Seg::Move(p) | Seg::Line(p) => *p,
        _ => return None,
    };
    let next = iter.find_map(|s| match *s {
        Seg::Line(p) => Some(p),
        Seg::Cubic(c1, _, p) => Some(if c1 == first { p } else { c1 }),
        _ => None,
    })?;
    Some((first, next.direction_to(first)))
}

/// Triangles covering a simple polygon (ear clipping), as index triples
/// into `points`. Works for concave outlines, which a painter that only
/// fills convex shapes needs.
pub fn triangulate(points: &[Point]) -> Vec<[usize; 3]> {
    let n = points.len();
    if n < 3 {
        return Vec::new();
    }
    let area: f32 = (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    let ccw = area > 0.0;
    let mut idx: Vec<usize> = (0..n).collect();
    let mut out = Vec::with_capacity(n - 2);
    let cross =
        |a: Point, b: Point, c: Point| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let mut guard = 0;
    while idx.len() > 3 && guard < n * n {
        guard += 1;
        let m = idx.len();
        let mut clipped = false;
        for i in 0..m {
            let (ia, ib, ic) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            let (a, b, c) = (points[ia], points[ib], points[ic]);
            let turn = cross(a, b, c);
            if (turn > 0.0) != ccw || turn == 0.0 {
                continue; // reflex or flat: not an ear
            }
            let inside = idx.iter().any(|&j| {
                if j == ia || j == ib || j == ic {
                    return false;
                }
                let p = points[j];
                let (d1, d2, d3) = (cross(a, b, p), cross(b, c, p), cross(c, a, p));
                if ccw {
                    d1 >= 0.0 && d2 >= 0.0 && d3 >= 0.0
                } else {
                    d1 <= 0.0 && d2 <= 0.0 && d3 <= 0.0
                }
            });
            if inside {
                continue;
            }
            out.push([ia, ib, ic]);
            idx.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            // Degenerate input: fan out what's left rather than loop.
            break;
        }
    }
    for k in 1..idx.len().saturating_sub(1) {
        out.push([idx[0], idx[k], idx[k + 1]]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::{ellipse, polygon};

    #[test]
    fn curves_flatten_onto_the_curve() {
        let c = Point::new(50.0, 50.0);
        let polys = flatten(&ellipse(c, 20.0, 20.0), 0.25);
        assert_eq!(polys.len(), 1);
        assert!(polys[0].closed);
        for p in &polys[0].points {
            assert!((p.distance(c) - 20.0).abs() < 0.2, "{p:?}");
        }
    }

    #[test]
    fn concave_polygons_triangulate_fully() {
        // An L shape: area 3 unit squares (of 10).
        let p = Point::new;
        let pts = [
            p(0.0, 0.0),
            p(20.0, 0.0),
            p(20.0, 10.0),
            p(10.0, 10.0),
            p(10.0, 20.0),
            p(0.0, 20.0),
        ];
        let tris = triangulate(&pts);
        assert_eq!(tris.len(), 4);
        let area: f32 = tris
            .iter()
            .map(|t| {
                let (a, b, c) = (pts[t[0]], pts[t[1]], pts[t[2]]);
                ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs() / 2.0
            })
            .sum();
        assert!((area - 300.0).abs() < 0.01, "{area}");
    }

    #[test]
    fn ends_and_directions() {
        let p = Point::new;
        let path = vec![
            Seg::Move(p(0.0, 0.0)),
            Seg::Line(p(10.0, 0.0)),
            Seg::Line(p(10.0, 10.0)),
        ];
        assert_eq!(end(&path), Some((p(10.0, 10.0), p(0.0, 1.0))));
        assert_eq!(start(&path), Some((p(0.0, 0.0), p(-1.0, 0.0))));
        assert_eq!(point_at(&path, 0.5), Some(p(10.0, 0.0)));
        assert!((length(&polygon(&[p(0.0, 0.0), p(3.0, 0.0), p(3.0, 4.0)])) - 7.0).abs() < 0.01);
    }
}
