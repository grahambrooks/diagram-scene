//! The drawing rules every painter shares: a shape's outline, where a
//! node's text goes, how big a node needs to be for its text, and the
//! marker at an edge's end.
//!
//! Text metrics here are fixed proportions of the font size (line height
//! 1.35×, ascent 0.8×), the same as [`Measure`]'s defaults, so a layout
//! engine and a painter agree on where lines fall without sharing a font.

use crate::geom::{Point, Rect};
use crate::{Anchor, Cardinality, Marker, Measure, Node, Seg, Shape, TextLine, line_width};

/// Baseline-to-baseline distance at `size` px.
pub fn line_height(size: f32) -> f32 {
    size * 1.35
}

/// Bézier handle length for a quarter circle of radius 1.
const KAPPA: f32 = 0.552_284_8;

/// Padding inside a table or class header and its compartments.
const HEADER_PAD: f32 = 6.0;
const COMPARTMENT_PAD: f32 = 4.0;
/// Rows in a compartment start this far from the left edge.
const ROW_INDENT: f32 = 8.0;

/// A shape, ready to draw.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Outline {
    /// Filled with the node's fill and stroked with its stroke. May hold
    /// several subpaths (a person's head and body).
    pub body: Vec<Seg>,
    /// Filled and stroked like the body, over it (component tabs).
    pub extras: Vec<Vec<Seg>>,
    /// Stroked only, over the body (a cylinder's rim, a note's fold).
    pub details: Vec<Vec<Seg>>,
}

/// The outline of `shape` drawn in `rect`.
pub fn outline(shape: Shape, rect: Rect) -> Outline {
    let Rect {
        x,
        y,
        width: w,
        height: h,
    } = rect;
    let (r, b) = (rect.right(), rect.bottom());
    let c = rect.center();
    let p = Point::new;
    let body = |segs: Vec<Seg>| Outline {
        body: segs,
        ..Outline::default()
    };
    match shape {
        Shape::Rect | Shape::ForkBar => body(polygon(&[p(x, y), p(r, y), p(r, b), p(x, b)])),
        Shape::Rounded => body(rounded_rect(rect, (h / 4.0).min(8.0))),
        Shape::Table | Shape::Class => body(rounded_rect(rect, 4.0)),
        Shape::Stadium => body(rounded_rect(rect, h / 2.0)),
        Shape::Circle | Shape::Start => {
            let radius = w.min(h) / 2.0;
            body(ellipse(c, radius, radius))
        }
        Shape::End => {
            let radius = w.min(h) / 2.0;
            Outline {
                body: ellipse(c, radius * 0.6, radius * 0.6),
                extras: Vec::new(),
                details: vec![ellipse(c, radius, radius)],
            }
        }
        Shape::Ellipse => body(ellipse(c, w / 2.0, h / 2.0)),
        Shape::Diamond => body(polygon(&[p(c.x, y), p(r, c.y), p(c.x, b), p(x, c.y)])),
        Shape::Hexagon => {
            let i = (h / 2.0).min(w / 4.0);
            body(polygon(&[
                p(x + i, y),
                p(r - i, y),
                p(r, c.y),
                p(r - i, b),
                p(x + i, b),
                p(x, c.y),
            ]))
        }
        Shape::Parallelogram => {
            let s = (h / 2.0).min(w / 5.0);
            body(polygon(&[p(x + s, y), p(r, y), p(r - s, b), p(x, b)]))
        }
        Shape::Trapezoid => {
            let s = (h / 2.0).min(w / 5.0);
            body(polygon(&[p(x + s, y), p(r - s, y), p(r, b), p(x, b)]))
        }
        Shape::Cylinder => {
            let ry = cylinder_rim(rect);
            let k = KAPPA * ry;
            let hw = w / 2.0;
            let kx = KAPPA * hw;
            let top = y + ry;
            let bottom = b - ry;
            let segs = vec![
                Seg::Move(p(x, top)),
                Seg::Cubic(p(x, top - k), p(c.x - kx, y), p(c.x, y)),
                Seg::Cubic(p(c.x + kx, y), p(r, top - k), p(r, top)),
                Seg::Line(p(r, bottom)),
                Seg::Cubic(p(r, bottom + k), p(c.x + kx, b), p(c.x, b)),
                Seg::Cubic(p(c.x - kx, b), p(x, bottom + k), p(x, bottom)),
                Seg::Close,
            ];
            // The rim's near side.
            let rim = vec![
                Seg::Move(p(x, top)),
                Seg::Cubic(p(x, top + k), p(c.x - kx, top + ry), p(c.x, top + ry)),
                Seg::Cubic(p(c.x + kx, top + ry), p(r, top + k), p(r, top)),
            ];
            Outline {
                body: segs,
                extras: Vec::new(),
                details: vec![rim],
            }
        }
        Shape::Queue => {
            let rx = queue_rim(rect);
            let k = KAPPA * rx;
            let hh = h / 2.0;
            let ky = KAPPA * hh;
            let left = x + rx;
            let right = r - rx;
            let segs = vec![
                Seg::Move(p(left, y)),
                Seg::Line(p(right, y)),
                Seg::Cubic(p(right + k, y), p(r, c.y - ky), p(r, c.y)),
                Seg::Cubic(p(r, c.y + ky), p(right + k, b), p(right, b)),
                Seg::Line(p(left, b)),
                Seg::Cubic(p(left - k, b), p(x, c.y + ky), p(x, c.y)),
                Seg::Cubic(p(x, c.y - ky), p(left - k, y), p(left, y)),
                Seg::Close,
            ];
            let rim = vec![
                Seg::Move(p(right, y)),
                Seg::Cubic(p(right - k, y), p(right - rx, c.y - ky), p(right - rx, c.y)),
                Seg::Cubic(p(right - rx, c.y + ky), p(right - k, b), p(right, b)),
            ];
            Outline {
                body: segs,
                extras: Vec::new(),
                details: vec![rim],
            }
        }
        Shape::Document => {
            let a = (h * 0.08).min(6.0);
            body(vec![
                Seg::Move(p(x, y)),
                Seg::Line(p(r, y)),
                Seg::Line(p(r, b - a)),
                Seg::Cubic(
                    p(r - w * 0.3, b + a),
                    p(x + w * 0.3, b - 3.0 * a),
                    p(x, b - a),
                ),
                Seg::Close,
            ])
        }
        Shape::Subroutine => {
            let i = 8.0_f32.min(w / 6.0);
            Outline {
                body: polygon(&[p(x, y), p(r, y), p(r, b), p(x, b)]),
                extras: Vec::new(),
                details: vec![
                    vec![Seg::Move(p(x + i, y)), Seg::Line(p(x + i, b))],
                    vec![Seg::Move(p(r - i, y)), Seg::Line(p(r - i, b))],
                ],
            }
        }
        Shape::Person => {
            // The torso, then the head over its shoulders.
            let (head_c, head_r, torso) = person_parts(rect);
            let mut segs = rounded_rect(torso, head_r.min(torso.height / 2.0));
            segs.extend(ellipse(head_c, head_r, head_r));
            body(segs)
        }
        Shape::Actor => {
            let fig = actor_figure(rect);
            let head_r = fig.width.min(fig.height) * 0.16;
            let head_c = p(fig.center().x, fig.y + head_r);
            let neck = head_c.y + head_r;
            let hip = fig.y + fig.height * 0.66;
            let arms = neck + (hip - neck) * 0.3;
            let half = fig.width * 0.32;
            let line = |a: Point, b: Point| vec![Seg::Move(a), Seg::Line(b)];
            Outline {
                body: ellipse(head_c, head_r, head_r),
                extras: Vec::new(),
                details: vec![
                    line(p(head_c.x, neck), p(head_c.x, hip)),
                    line(p(head_c.x - half, arms), p(head_c.x + half, arms)),
                    line(p(head_c.x, hip), p(head_c.x - half, fig.bottom())),
                    line(p(head_c.x, hip), p(head_c.x + half, fig.bottom())),
                ],
            }
        }
        Shape::Note => {
            let f = 10.0_f32.min(w / 4.0).min(h / 4.0);
            Outline {
                body: polygon(&[p(x, y), p(r - f, y), p(r, y + f), p(r, b), p(x, b)]),
                extras: Vec::new(),
                details: vec![vec![
                    Seg::Move(p(r - f, y)),
                    Seg::Line(p(r - f, y + f)),
                    Seg::Line(p(r, y + f)),
                ]],
            }
        }
        Shape::Component => {
            let tab = |ty: f32| {
                polygon(&[
                    p(x - 6.0, ty),
                    p(x + 6.0, ty),
                    p(x + 6.0, ty + 7.0),
                    p(x - 6.0, ty + 7.0),
                ])
            };
            Outline {
                body: rounded_rect(rect, 3.0),
                extras: vec![tab(y + h * 0.25 - 3.5), tab(y + h * 0.6 - 3.5)],
                details: Vec::new(),
            }
        }
        Shape::Package => {
            let (tw, th) = package_tab(rect);
            body(polygon(&[
                p(x, y),
                p(x + tw, y),
                p(x + tw + 4.0, y + th),
                p(r, y + th),
                p(r, b),
                p(x, b),
            ]))
        }
        Shape::Cloud => body(cloud(rect)),
    }
}

/// The text inside (or beside) a node, placed.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedText {
    /// The baseline point `anchor` refers to.
    pub pos: Point,
    pub anchor: Anchor,
    pub line: TextLine,
    /// Drawn in the muted colour (stereotypes, technology, rows' types).
    pub muted: bool,
}

/// A node's text and the rules between a table's or class's sections.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextLayout {
    pub lines: Vec<PlacedText>,
    pub rules: Vec<(Point, Point)>,
}

/// Where `node`'s text goes, at body size `base`.
pub fn text_layout(node: &Node, base: f32) -> TextLayout {
    let mut out = TextLayout::default();
    match node.shape {
        Shape::Table | Shape::Class => {
            let rect = node.rect;
            let mut top = rect.y + HEADER_PAD;
            let header: Vec<(&TextLine, bool)> = node
                .stereotype
                .iter()
                .map(|l| (l, true))
                .chain(node.label.iter().map(|l| (l, false)))
                .collect();
            for (line, muted) in header {
                let lh = line_height(line.font().size(base));
                out.lines.push(PlacedText {
                    pos: Point::new(rect.center().x, baseline(top, line, base)),
                    anchor: Anchor::Middle,
                    line: line.clone(),
                    muted,
                });
                top += lh;
            }
            top += HEADER_PAD;
            for section in &node.compartments {
                out.rules
                    .push((Point::new(rect.x, top), Point::new(rect.right(), top)));
                top += COMPARTMENT_PAD;
                for line in section {
                    out.lines.push(PlacedText {
                        pos: Point::new(rect.x + ROW_INDENT, baseline(top, line, base)),
                        anchor: Anchor::Start,
                        line: line.clone(),
                        muted: false,
                    });
                    top += line_height(line.font().size(base));
                }
                top += COMPARTMENT_PAD.max(if section.is_empty() { 6.0 } else { 0.0 });
            }
        }
        _ => {
            let area = text_area(node.shape, node.rect, base);
            let block: Vec<(&TextLine, bool)> = node
                .stereotype
                .iter()
                .map(|l| (l, true))
                .chain(node.label.iter().map(|l| (l, false)))
                .chain(node.sublabel.iter().map(|l| (l, true)))
                .collect();
            let total: f32 = block
                .iter()
                .map(|(l, _)| line_height(l.font().size(base)))
                .sum();
            let mut top = area.center().y - total / 2.0;
            for (line, muted) in block {
                out.lines.push(PlacedText {
                    pos: Point::new(area.center().x, baseline(top, line, base)),
                    anchor: Anchor::Middle,
                    line: line.clone(),
                    muted,
                });
                top += line_height(line.font().size(base));
            }
        }
    }
    out
}

/// The baseline of `line` in a line box starting at `top`.
fn baseline(top: f32, line: &TextLine, base: f32) -> f32 {
    let size = line.font().size(base);
    top + (line_height(size) - size) / 2.0 + size * 0.8
}

/// Where a centred shape's text block is centred.
fn text_area(shape: Shape, rect: Rect, base: f32) -> Rect {
    match shape {
        // Below the head, which overlaps the torso's top.
        Shape::Person => {
            let below = rect.y + person_head(rect) * 2.0 + 2.0;
            Rect::new(rect.x, below, rect.width, rect.bottom() - below)
        }
        Shape::Actor => {
            let fig = actor_figure(rect);
            Rect::new(
                rect.x,
                fig.bottom(),
                rect.width,
                rect.bottom() - fig.bottom(),
            )
        }
        Shape::Cylinder => {
            let ry = cylinder_rim(rect);
            Rect::new(
                rect.x,
                rect.y + 2.0 * ry,
                rect.width,
                rect.height - 3.0 * ry,
            )
        }
        Shape::Document => Rect::new(rect.x, rect.y, rect.width, rect.height * 0.92),
        Shape::Package => {
            let (_, th) = package_tab(rect);
            Rect::new(rect.x, rect.y + th, rect.width, rect.height - th)
        }
        // Dots and bars carry their label underneath.
        Shape::Start | Shape::End | Shape::ForkBar => Rect::new(
            rect.x - 60.0,
            rect.bottom() + 2.0,
            rect.width + 120.0,
            line_height(base),
        ),
        _ => rect,
    }
}

/// How big `node` needs to be to hold its text, at body size `base`.
pub fn natural_size(node: &Node, base: f32, measure: &dyn Measure) -> (f32, f32) {
    let widest = |lines: &[TextLine]| {
        lines
            .iter()
            .map(|l| line_width(measure, l, base))
            .fold(0.0_f32, f32::max)
    };
    let height = |lines: &[TextLine]| -> f32 {
        lines.iter().map(|l| line_height(l.font().size(base))).sum()
    };
    if matches!(node.shape, Shape::Table | Shape::Class) {
        let header_w = widest(&node.stereotype).max(widest(&node.label));
        let rows_w = node
            .compartments
            .iter()
            .map(|c| widest(c) + 2.0 * ROW_INDENT)
            .fold(0.0_f32, f32::max);
        let mut h = 2.0 * HEADER_PAD + height(&node.stereotype) + height(&node.label);
        for section in &node.compartments {
            h += height(section)
                + COMPARTMENT_PAD
                + COMPARTMENT_PAD.max(if section.is_empty() { 6.0 } else { 0.0 });
        }
        return ((header_w + 24.0).max(rows_w).max(80.0), h);
    }
    let w = widest(&node.stereotype)
        .max(widest(&node.label))
        .max(widest(&node.sublabel));
    let h = height(&node.stereotype) + height(&node.label) + height(&node.sublabel);
    match node.shape {
        Shape::Rect | Shape::Rounded | Shape::Note => (w + 28.0, h + 20.0),
        Shape::Stadium => (w + h + 24.0, h + 18.0),
        Shape::Circle => {
            let d = w.max(h) + 24.0;
            (d, d)
        }
        Shape::Ellipse => (w * 1.3 + 24.0, h * 1.4 + 18.0),
        Shape::Diamond => (w * 1.6 + 30.0, h * 1.6 + 24.0),
        Shape::Hexagon => (w + h + 32.0, h + 20.0),
        Shape::Parallelogram | Shape::Trapezoid => (w + h + 32.0, h + 20.0),
        Shape::Cylinder => (w + 28.0, h + 44.0),
        Shape::Document => (w + 28.0, h + 28.0),
        Shape::Subroutine => (w + 44.0, h + 20.0),
        Shape::Person => ((w + 28.0).max(110.0), h + 20.0 + 2.0 * PERSON_HEAD + 2.0),
        Shape::Actor => ((w + 8.0).max(40.0), ((h + 4.0) / 0.32).max(60.0)),
        Shape::Component => (w + 44.0, h + 24.0),
        Shape::Package => (w + 28.0, h + 34.0),
        Shape::Queue => (w + 52.0, h + 20.0),
        Shape::Cloud => (w * 1.4 + 32.0, h * 1.6 + 28.0),
        Shape::Start => (20.0, 20.0),
        Shape::End => (22.0, 22.0),
        Shape::ForkBar => (80.0, 6.0),
        Shape::Table | Shape::Class => unreachable!(),
    }
}

/// A marker, ready to draw.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MarkerShape {
    pub parts: Vec<MarkerPart>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MarkerPart {
    pub path: Vec<Seg>,
    pub fill: MarkerFill,
}

/// How a marker part is filled; every part is stroked with the edge's pen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerFill {
    None,
    /// The edge's colour.
    Line,
    /// The background, hiding the line under a hollow marker.
    Background,
}

/// `marker` with its tip at `tip`, the edge arriving along `dir` (a unit
/// vector pointing into the tip), for a pen `width` wide.
pub fn marker(marker: Marker, tip: Point, dir: Point, width: f32) -> MarkerShape {
    let s = 9.0 + 2.0 * width;
    let back = |d: f32| Point::new(tip.x - dir.x * d, tip.y - dir.y * d);
    let side = |pt: Point, d: f32| Point::new(pt.x - dir.y * d, pt.y + dir.x * d);
    let part = |path: Vec<Seg>, fill| MarkerPart { path, fill };
    let line = |a: Point, b: Point| vec![Seg::Move(a), Seg::Line(b)];
    let circle_at = |d: f32, r: f32| part(ellipse(back(d), r, r), MarkerFill::Background);
    let bar_at = |d: f32| {
        part(
            line(side(back(d), s * 0.45), side(back(d), -s * 0.45)),
            MarkerFill::None,
        )
    };
    let foot = || {
        let root = back(s);
        vec![
            part(line(root, side(tip, s * 0.5)), MarkerFill::None),
            part(line(root, tip), MarkerFill::None),
            part(line(root, side(tip, -s * 0.5)), MarkerFill::None),
        ]
    };
    let parts = match marker {
        Marker::None => Vec::new(),
        Marker::Arrow => vec![part(
            polygon(&[tip, side(back(s), s * 0.4), side(back(s), -s * 0.4)]),
            MarkerFill::Line,
        )],
        Marker::OpenArrow => vec![part(
            vec![
                Seg::Move(side(back(s), s * 0.45)),
                Seg::Line(tip),
                Seg::Line(side(back(s), -s * 0.45)),
            ],
            MarkerFill::None,
        )],
        Marker::HollowTriangle => {
            let s = s * 1.25;
            vec![part(
                polygon(&[tip, side(back(s), s * 0.5), side(back(s), -s * 0.5)]),
                MarkerFill::Background,
            )]
        }
        Marker::FilledDiamond | Marker::HollowDiamond => {
            let fill = if marker == Marker::FilledDiamond {
                MarkerFill::Line
            } else {
                MarkerFill::Background
            };
            vec![part(
                polygon(&[
                    tip,
                    side(back(s * 0.75), s * 0.4),
                    back(s * 1.5),
                    side(back(s * 0.75), -s * 0.4),
                ]),
                fill,
            )]
        }
        Marker::Circle => vec![circle_at(4.5, 4.5)],
        Marker::Cross => {
            let c = back(6.0);
            let d = 4.5;
            vec![
                part(line(c.offset(-d, -d), c.offset(d, d)), MarkerFill::None),
                part(line(c.offset(-d, d), c.offset(d, -d)), MarkerFill::None),
            ]
        }
        Marker::CrowFoot(card) => match card {
            Cardinality::One => vec![bar_at(s * 0.45), bar_at(s * 0.8)],
            Cardinality::ZeroOrOne => vec![bar_at(s * 0.45), circle_at(s * 1.2, 4.0)],
            Cardinality::Many => foot(),
            Cardinality::OneOrMany => {
                let mut parts = foot();
                parts.push(bar_at(s * 1.25));
                parts
            }
            Cardinality::ZeroOrMany => {
                let mut parts = foot();
                parts.push(circle_at(s * 1.45, 4.0));
                parts
            }
        },
    };
    MarkerShape { parts }
}

/// A closed polygon through `points`.
pub fn polygon(points: &[Point]) -> Vec<Seg> {
    let mut segs = Vec::with_capacity(points.len() + 1);
    for (i, &pt) in points.iter().enumerate() {
        segs.push(if i == 0 { Seg::Move(pt) } else { Seg::Line(pt) });
    }
    segs.push(Seg::Close);
    segs
}

/// A rectangle with corners rounded to `radius` (clamped to fit).
pub fn rounded_rect(rect: Rect, radius: f32) -> Vec<Seg> {
    let r = radius.min(rect.width / 2.0).min(rect.height / 2.0).max(0.0);
    let (x, y, right, bottom) = (rect.x, rect.y, rect.right(), rect.bottom());
    if r == 0.0 {
        return polygon(&[
            Point::new(x, y),
            Point::new(right, y),
            Point::new(right, bottom),
            Point::new(x, bottom),
        ]);
    }
    let k = r * (1.0 - KAPPA);
    let p = Point::new;
    vec![
        Seg::Move(p(x + r, y)),
        Seg::Line(p(right - r, y)),
        Seg::Cubic(p(right - k, y), p(right, y + k), p(right, y + r)),
        Seg::Line(p(right, bottom - r)),
        Seg::Cubic(
            p(right, bottom - k),
            p(right - k, bottom),
            p(right - r, bottom),
        ),
        Seg::Line(p(x + r, bottom)),
        Seg::Cubic(p(x + k, bottom), p(x, bottom - k), p(x, bottom - r)),
        Seg::Line(p(x, y + r)),
        Seg::Cubic(p(x, y + k), p(x + k, y), p(x + r, y)),
        Seg::Close,
    ]
}

/// An ellipse as four cubic arcs.
pub fn ellipse(c: Point, rx: f32, ry: f32) -> Vec<Seg> {
    let (kx, ky) = (rx * KAPPA, ry * KAPPA);
    let p = Point::new;
    vec![
        Seg::Move(p(c.x + rx, c.y)),
        Seg::Cubic(
            p(c.x + rx, c.y + ky),
            p(c.x + kx, c.y + ry),
            p(c.x, c.y + ry),
        ),
        Seg::Cubic(
            p(c.x - kx, c.y + ry),
            p(c.x - rx, c.y + ky),
            p(c.x - rx, c.y),
        ),
        Seg::Cubic(
            p(c.x - rx, c.y - ky),
            p(c.x - kx, c.y - ry),
            p(c.x, c.y - ry),
        ),
        Seg::Cubic(
            p(c.x + kx, c.y - ry),
            p(c.x + rx, c.y - ky),
            p(c.x + rx, c.y),
        ),
        Seg::Close,
    ]
}

fn cylinder_rim(rect: Rect) -> f32 {
    (rect.height * 0.12).min(rect.width * 0.2).min(10.0)
}

fn queue_rim(rect: Rect) -> f32 {
    (rect.width * 0.12).min(rect.height * 0.2).min(10.0)
}

fn package_tab(rect: Rect) -> (f32, f32) {
    ((rect.width * 0.4).min(60.0), (rect.height * 0.2).min(12.0))
}

/// A person's head (centre, radius) and torso.
fn person_parts(rect: Rect) -> (Point, f32, Rect) {
    let r = person_head(rect);
    let head = Point::new(rect.center().x, rect.y + r);
    let top = rect.y + r * 1.6;
    (
        head,
        r,
        Rect::new(rect.x, top, rect.width, rect.bottom() - top),
    )
}

/// A person's head radius.
fn person_head(rect: Rect) -> f32 {
    (rect.height * 0.2).min(rect.width * 0.2).min(PERSON_HEAD)
}

/// The largest head a person gets.
const PERSON_HEAD: f32 = 18.0;

/// The part of an actor's rectangle its stick figure takes.
fn actor_figure(rect: Rect) -> Rect {
    let h = rect.height * 0.66;
    let w = (h * 0.6).min(rect.width);
    Rect::new(rect.center().x - w / 2.0, rect.y, w, h)
}

/// A cloud: bumps around an ellipse.
fn cloud(rect: Rect) -> Vec<Seg> {
    const BUMPS: usize = 9;
    let c = rect.center();
    let (rx, ry) = (rect.width / 2.0 * 0.86, rect.height / 2.0 * 0.8);
    let at = |i: usize| {
        let a = i as f32 / BUMPS as f32 * std::f32::consts::TAU;
        Point::new(c.x + rx * a.cos(), c.y + ry * a.sin())
    };
    let mut segs = vec![Seg::Move(at(0))];
    for i in 0..BUMPS {
        let (a, b) = (at(i), at(i + 1));
        // Push the control points outwards from the centre.
        let out = |p: Point| {
            let d = c.direction_to(p);
            Point::new(p.x + d.x * rx * 0.35, p.y + d.y * ry * 0.35)
        };
        segs.push(Seg::Cubic(out(a.lerp(b, 0.15)), out(a.lerp(b, 0.85)), b));
    }
    segs.push(Seg::Close);
    segs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Estimate, Font, Node};

    fn bounds_of(segs: &[Seg]) -> Rect {
        let pts = crate::flatten::flatten(segs, 0.5);
        let mut b: Option<Rect> = None;
        for poly in pts {
            for p in poly.points {
                let r = Rect::new(p.x, p.y, 0.0, 0.0);
                b = Some(b.map_or(r, |b| b.union(&r)));
            }
        }
        b.unwrap()
    }

    #[test]
    fn outlines_stay_in_their_rectangles() {
        let rect = Rect::new(10.0, 20.0, 120.0, 60.0);
        for shape in ALL {
            if matches!(shape, Shape::Component | Shape::Cloud) {
                continue; // tabs and bumps reach a little outside, on purpose
            }
            let o = outline(shape, rect);
            assert!(
                rect.encloses(&bounds_of(&o.body), 0.6),
                "{shape:?}: {:?}",
                bounds_of(&o.body)
            );
        }
    }

    #[test]
    fn text_is_centred_and_fits() {
        let node = Node::new("n", Shape::Rounded, Rect::default()).with_label("Ledger DB");
        let (w, h) = natural_size(&node, 14.0, &Estimate);
        let node = Node {
            rect: Rect::new(0.0, 0.0, w, h),
            ..node
        };
        let layout = text_layout(&node, 14.0);
        assert_eq!(layout.lines.len(), 1);
        let t = &layout.lines[0];
        assert!((t.pos.x - w / 2.0).abs() < 0.01);
        assert!(t.pos.y > 0.0 && t.pos.y < h);
    }

    #[test]
    fn classes_rule_off_their_compartments() {
        let mut node = Node::new("c", Shape::Class, Rect::default()).with_label("Order");
        node.compartments = vec![
            vec![TextLine::new("id: Uuid", Font::Mono)],
            vec![TextLine::new("total(): Money", Font::Mono)],
        ];
        let (w, h) = natural_size(&node, 14.0, &Estimate);
        node.rect = Rect::new(0.0, 0.0, w, h);
        let layout = text_layout(&node, 14.0);
        assert_eq!(layout.rules.len(), 2);
        assert_eq!(layout.lines.len(), 3);
        let last = layout.lines.last().unwrap();
        assert!(last.pos.y < h, "{} within {h}", last.pos.y);
        assert!(layout.rules[0].0.y < layout.lines[1].pos.y);
    }

    #[test]
    fn markers_end_at_the_tip() {
        let tip = Point::new(100.0, 50.0);
        let dir = Point::new(1.0, 0.0);
        for m in [
            Marker::Arrow,
            Marker::OpenArrow,
            Marker::HollowTriangle,
            Marker::FilledDiamond,
            Marker::HollowDiamond,
            Marker::CrowFoot(Cardinality::OneOrMany),
        ] {
            let shape = marker(m, tip, dir, 1.4);
            let reach = shape
                .parts
                .iter()
                .map(|p| bounds_of(&p.path).right())
                .fold(f32::MIN, f32::max);
            assert!((reach - tip.x).abs() < 0.6, "{m:?} reaches {reach}");
        }
    }

    const ALL: [Shape; 24] = [
        Shape::Rect,
        Shape::Rounded,
        Shape::Stadium,
        Shape::Circle,
        Shape::Ellipse,
        Shape::Diamond,
        Shape::Hexagon,
        Shape::Parallelogram,
        Shape::Trapezoid,
        Shape::Cylinder,
        Shape::Document,
        Shape::Subroutine,
        Shape::Person,
        Shape::Actor,
        Shape::Note,
        Shape::Component,
        Shape::Package,
        Shape::Queue,
        Shape::Cloud,
        Shape::Start,
        Shape::End,
        Shape::ForkBar,
        Shape::Table,
        Shape::Class,
    ];
}
