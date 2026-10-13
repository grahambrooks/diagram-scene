//! Diagram-space geometry. Units are px; y grows downwards.

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance(self, other: Point) -> f32 {
        (other.x - self.x).hypot(other.y - self.y)
    }

    /// The point a fraction `t` of the way to `other`.
    pub fn lerp(self, other: Point, t: f32) -> Point {
        Point::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
        )
    }

    /// The unit vector from `self` towards `other` (zero if they meet).
    pub fn direction_to(self, other: Point) -> Point {
        let d = self.distance(other);
        if d == 0.0 {
            Point::default()
        } else {
            Point::new((other.x - self.x) / d, (other.y - self.y) / d)
        }
    }

    pub fn offset(self, dx: f32, dy: f32) -> Point {
        Point::new(self.x + dx, self.y + dy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// The rectangle of `size` centred on `center`.
    pub fn centered(center: Point, width: f32, height: f32) -> Self {
        Self::new(
            center.x - width / 2.0,
            center.y - height / 2.0,
            width,
            height,
        )
    }

    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Grow the rectangle by `amount` on every side.
    pub fn inflate(&self, amount: f32) -> Rect {
        Rect::new(
            self.x - amount,
            self.y - amount,
            self.width + 2.0 * amount,
            self.height + 2.0 * amount,
        )
    }

    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Rect::new(
            x,
            y,
            self.right().max(other.right()) - x,
            self.bottom().max(other.bottom()) - y,
        )
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x <= self.right() && p.y >= self.y && p.y <= self.bottom()
    }

    /// Whether the interiors overlap (touching edges don't count).
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    /// Whether `other` lies inside this rectangle, give or take `slack`.
    pub fn encloses(&self, other: &Rect, slack: f32) -> bool {
        other.x >= self.x - slack
            && other.y >= self.y - slack
            && other.right() <= self.right() + slack
            && other.bottom() <= self.bottom() + slack
    }
}
