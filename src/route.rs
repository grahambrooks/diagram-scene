//! Edge routing shared by the layout engines: where an edge meets a
//! shape, and the path between two nodes.

use crate::geom::{Point, Rect};
use crate::{Node, Seg, Shape};

/// A side of a rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

/// The middle of `side` of `rect`.
pub fn port(rect: Rect, side: Side) -> Point {
    let c = rect.center();
    match side {
        Side::Top => Point::new(c.x, rect.y),
        Side::Right => Point::new(rect.right(), c.y),
        Side::Bottom => Point::new(c.x, rect.bottom()),
        Side::Left => Point::new(rect.x, c.y),
    }
}

/// Where a line from the centre of `shape` in `rect` towards `toward`
/// leaves the shape's outline.
pub fn clip(shape: Shape, rect: Rect, toward: Point) -> Point {
    let c = rect.center();
    let (dx, dy) = (toward.x - c.x, toward.y - c.y);
    if dx == 0.0 && dy == 0.0 {
        return c;
    }
    let (hw, hh) = (rect.width / 2.0, rect.height / 2.0);
    let t = match shape {
        Shape::Circle | Shape::Start | Shape::End => {
            let r = hw.min(hh);
            r / dx.hypot(dy)
        }
        Shape::Ellipse | Shape::Cloud => 1.0 / ((dx / hw).powi(2) + (dy / hh).powi(2)).sqrt(),
        Shape::Diamond => 1.0 / (dx.abs() / hw + dy.abs() / hh),
        _ => {
            let tx = if dx == 0.0 {
                f32::INFINITY
            } else {
                hw / dx.abs()
            };
            let ty = if dy == 0.0 {
                f32::INFINITY
            } else {
                hh / dy.abs()
            };
            tx.min(ty)
        }
    };
    Point::new(c.x + dx * t, c.y + dy * t)
}

/// A straight edge between two nodes, from outline to outline.
pub fn straight(from: &Node, to: &Node) -> Vec<Seg> {
    let (a, b) = (from.rect.center(), to.rect.center());
    vec![
        Seg::Move(clip(from.shape, from.rect, b)),
        Seg::Line(clip(to.shape, to.rect, a)),
    ]
}

/// The sides two rectangles face each other on, given the flow's main
/// axis (`horizontal` for left-right layouts).
pub fn facing(from: Rect, to: Rect, horizontal: bool) -> (Side, Side) {
    let (a, b) = (from.center(), to.center());
    if horizontal {
        if b.x >= a.x {
            (Side::Right, Side::Left)
        } else {
            (Side::Left, Side::Right)
        }
    } else if b.y >= a.y {
        (Side::Bottom, Side::Top)
    } else {
        (Side::Top, Side::Bottom)
    }
}

/// A smooth curve between facing sides, leaving and arriving square to
/// them (the call graph's edges).
pub fn spline(from: Rect, to: Rect, horizontal: bool) -> Vec<Seg> {
    let (sa, sb) = facing(from, to, horizontal);
    let (a, b) = (port(from, sa), port(to, sb));
    curve_between(a, sa, b, sb)
}

/// A cubic from `a` (leaving `side_a` outwards) to `b` (arriving through
/// `side_b`).
pub fn curve_between(a: Point, side_a: Side, b: Point, side_b: Side) -> Vec<Seg> {
    let span = match side_a {
        Side::Left | Side::Right => (b.x - a.x).abs(),
        Side::Top | Side::Bottom => (b.y - a.y).abs(),
    };
    let bend = (span / 2.0).max(24.0);
    let out = |p: Point, side: Side| match side {
        Side::Top => p.offset(0.0, -bend),
        Side::Right => p.offset(bend, 0.0),
        Side::Bottom => p.offset(0.0, bend),
        Side::Left => p.offset(-bend, 0.0),
    };
    vec![Seg::Move(a), Seg::Cubic(out(a, side_a), out(b, side_b), b)]
}

/// A right-angled route between facing sides: out, across at the midway
/// line, and in, with corners rounded to `radius`.
pub fn orthogonal(from: Rect, to: Rect, horizontal: bool, radius: f32) -> Vec<Seg> {
    let (sa, sb) = facing(from, to, horizontal);
    let (a, b) = (port(from, sa), port(to, sb));
    let points = if horizontal {
        let mx = (a.x + b.x) / 2.0;
        vec![a, Point::new(mx, a.y), Point::new(mx, b.y), b]
    } else {
        let my = (a.y + b.y) / 2.0;
        vec![a, Point::new(a.x, my), Point::new(b.x, my), b]
    };
    rounded_polyline(&points, radius)
}

/// A polyline through `points` with its corners rounded to `radius`
/// (less where the legs are short). Collinear points pass straight.
pub fn rounded_polyline(points: &[Point], radius: f32) -> Vec<Seg> {
    let pts: Vec<Point> = points
        .iter()
        .copied()
        .fold(Vec::new(), |mut v: Vec<Point>, p| {
            if v.last() != Some(&p) {
                v.push(p);
            }
            v
        });
    let Some(&first) = pts.first() else {
        return Vec::new();
    };
    let mut segs = vec![Seg::Move(first)];
    for i in 1..pts.len() {
        let p = pts[i];
        if i + 1 == pts.len() {
            segs.push(Seg::Line(p));
            break;
        }
        let (prev, next) = (pts[i - 1], pts[i + 1]);
        let r = radius
            .min(prev.distance(p) / 2.0)
            .min(p.distance(next) / 2.0);
        let (din, dout) = (prev.direction_to(p), p.direction_to(next));
        let straight = (din.x * dout.y - din.y * dout.x).abs() < 1e-3;
        if r <= 0.0 || straight {
            segs.push(Seg::Line(p));
            continue;
        }
        let before = p.offset(-din.x * r, -din.y * r);
        let after = p.offset(dout.x * r, dout.y * r);
        segs.push(Seg::Line(before));
        let k = 0.447_715; // 1 - KAPPA: handles for a quarter circle
        segs.push(Seg::Cubic(
            before.lerp(p, 1.0 - k),
            after.lerp(p, 1.0 - k),
            after,
        ));
    }
    segs
}

/// Where a label sits on a path: its midpoint, nudged `offset` px to the
/// left of the direction of travel (0 for on the line).
pub fn label_position(path: &[Seg], offset: f32) -> Option<Point> {
    let mid = crate::flatten::point_at(path, 0.5)?;
    if offset == 0.0 {
        return Some(mid);
    }
    let ahead = crate::flatten::point_at(path, 0.52)?;
    let d = mid.direction_to(ahead);
    Some(Point::new(mid.x + d.y * offset, mid.y - d.x * offset))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipping_meets_each_outline() {
        let rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let right = Point::new(500.0, 20.0);
        assert_eq!(clip(Shape::Rounded, rect, right), Point::new(100.0, 20.0));
        assert_eq!(clip(Shape::Diamond, rect, right), Point::new(100.0, 20.0));
        let up = Point::new(50.0, -500.0);
        assert_eq!(clip(Shape::Rect, rect, up), Point::new(50.0, 0.0));
        // Diagonally, a diamond's edge is nearer than the box's corner.
        let diag = Point::new(150.0, 60.0);
        let d = clip(Shape::Diamond, rect, diag);
        let b = clip(Shape::Rect, rect, diag);
        assert!(d.distance(rect.center()) < b.distance(rect.center()));
        let circle = Rect::new(0.0, 0.0, 40.0, 40.0);
        let c = clip(Shape::Circle, circle, Point::new(100.0, 100.0));
        assert!((c.distance(circle.center()) - 20.0).abs() < 1e-3);
    }

    #[test]
    fn orthogonal_routes_turn_twice_with_rounded_corners() {
        let a = Rect::new(0.0, 0.0, 40.0, 20.0);
        let b = Rect::new(100.0, 100.0, 40.0, 20.0);
        let path = orthogonal(a, b, false, 6.0);
        assert_eq!(path.first(), Some(&Seg::Move(Point::new(20.0, 20.0))));
        assert_eq!(path.last(), Some(&Seg::Line(Point::new(120.0, 100.0))));
        let corners = path.iter().filter(|s| matches!(s, Seg::Cubic(..))).count();
        assert_eq!(corners, 2);
    }

    #[test]
    fn splines_leave_and_arrive_square() {
        let a = Rect::new(0.0, 0.0, 40.0, 20.0);
        let b = Rect::new(200.0, 80.0, 40.0, 20.0);
        let path = spline(a, b, true);
        let Seg::Cubic(c1, c2, end) = path[1] else {
            panic!("{path:?}")
        };
        assert_eq!(c1.y, 10.0);
        assert_eq!(c2.y, end.y);
        assert_eq!(end, Point::new(200.0, 90.0));
    }
}
