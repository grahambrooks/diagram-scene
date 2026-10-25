//! A scene as an SVG document, in a theme. Plain attributes, no CSS, so
//! any SVG renderer (browsers, resvg, PDF converters) draws it the same.

use std::fmt::Write;

use crate::flatten;
use crate::shape::{self, MarkerFill};
use crate::theme::{Theme, text_paint};
use crate::{Anchor, ContainerKind, Dash, Edge, Font, Paint, Role, Scene, Seg, TextLine};

/// `scene` as SVG, coloured by `theme`.
pub fn to_svg(scene: &Scene, theme: &Theme) -> String {
    let mut out = String::new();
    let (w, h) = (scene.width.ceil(), scene.height.ceil());
    let _ = write!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}" font-family="{}" role="img">"#,
        attr(&theme.font_family)
    );
    // First children, so assistive technology finds the accessible name.
    if let Some(title) = &scene.accessibility.title {
        let _ = write!(out, "<title>{}</title>", escape(title));
    }
    if let Some(desc) = &scene.accessibility.description {
        let _ = write!(out, "<desc>{}</desc>", escape(desc));
    }
    let _ = write!(
        out,
        r#"<rect width="{w}" height="{h}" fill="{}"/>"#,
        theme.background
    );
    let base = scene.font_size;
    for c in &scene.containers {
        container(&mut out, theme, c, base);
    }
    for a in &scene.areas {
        area(&mut out, theme, a);
    }
    for e in &scene.edges {
        edge(&mut out, theme, e);
    }
    for n in &scene.nodes {
        node(&mut out, theme, n, base);
    }
    // Labels over the nodes, so a box never hides one.
    for e in &scene.edges {
        edge_text(&mut out, theme, e, base);
    }
    for t in &scene.texts {
        let rotated = t.rotate != 0.0;
        if rotated {
            let _ = write!(
                out,
                r#"<g transform="rotate({} {} {})">"#,
                r(t.rotate),
                r(t.pos.x),
                r(t.pos.y)
            );
        }
        if t.backed {
            let w = crate::line_width(&crate::Estimate, &t.line, base) + 6.0;
            let size = t.line.font().size(base);
            let x = match t.anchor {
                Anchor::Start => t.pos.x - 3.0,
                Anchor::Middle => t.pos.x - w / 2.0,
                Anchor::End => t.pos.x - w + 3.0,
            };
            let backing = crate::Rect::new(x, t.pos.y - size, w, size * 1.3);
            path(
                &mut out,
                &shape::rounded_rect(backing, 2.0),
                theme.background.to_string(),
                None,
                theme,
            );
        }
        text(&mut out, theme, t.pos, t.anchor, &t.line, t.paint, base);
        if rotated {
            out.push_str("</g>");
        }
    }
    if let Some(title) = &scene.title {
        text(
            &mut out,
            theme,
            title.pos,
            title.anchor,
            &title.line,
            title.paint,
            base,
        );
    }
    if let Some(legend) = &scene.legend {
        legend_block(&mut out, theme, legend, base);
    }
    out.push_str("</svg>");
    out
}

fn container(out: &mut String, theme: &Theme, c: &crate::Container, base: f32) {
    match &c.kind {
        ContainerKind::Lifeline => {
            let x = c.rect.center().x;
            let _ = write!(
                out,
                r#"<line x1="{x}" y1="{}" x2="{x}" y2="{}" stroke="{}" stroke-width="1"{}/>"#,
                c.rect.y,
                c.rect.bottom(),
                color(theme, c.style.stroke),
                dash_attr(c.style.dash, 1.0)
            );
            return;
        }
        ContainerKind::Fragment { operator, guards } => {
            path(
                out,
                &shape::rounded_rect(c.rect, 3.0),
                fill_attr(theme, c.style.fill),
                Some((c.style.stroke, 1.0, Dash::Solid)),
                theme,
            );
            // The operator in a tab in the corner.
            let tab_w = 14.0 + operator.chars().count() as f32 * base * 0.6;
            let tab_h = base * 1.4;
            let (x, y) = (c.rect.x, c.rect.y);
            let tab = shape::polygon(&[
                crate::Point::new(x, y),
                crate::Point::new(x + tab_w, y),
                crate::Point::new(x + tab_w, y + tab_h - 5.0),
                crate::Point::new(x + tab_w - 5.0, y + tab_h),
                crate::Point::new(x, y + tab_h),
            ]);
            path(
                out,
                &tab,
                fill_attr(theme, Paint::Role(Role::Surface)),
                Some((c.style.stroke, 1.0, Dash::Solid)),
                theme,
            );
            let op = TextLine::new(operator.clone(), Font::Bold);
            text(
                out,
                theme,
                crate::Point::new(x + 7.0, y + tab_h * 0.72),
                Anchor::Start,
                &op,
                Paint::Role(Role::Text),
                base * 0.85,
            );
            for (gy, guard) in guards {
                if *gy > c.rect.y + 1.0 {
                    let _ = write!(
                        out,
                        r#"<line x1="{}" y1="{gy}" x2="{}" y2="{gy}" stroke="{}" stroke-width="1"{}/>"#,
                        c.rect.x,
                        c.rect.right(),
                        color(theme, c.style.stroke),
                        dash_attr(Dash::Dashed, 1.0)
                    );
                }
                let gy = if *gy > c.rect.y + 1.0 { *gy } else { y + tab_h };
                text(
                    out,
                    theme,
                    crate::Point::new(x + tab_w + 10.0, gy + base),
                    Anchor::Start,
                    guard,
                    Paint::Role(Role::Muted),
                    base * 0.85,
                );
            }
            return;
        }
        _ => {}
    }
    let radius = match c.kind {
        ContainerKind::Swimlane => 0.0,
        _ => 6.0,
    };
    let weight = if c.style.emphasis { 2.0 } else { 1.0 };
    path(
        out,
        &shape::rounded_rect(c.rect, radius),
        fill_attr(theme, c.style.fill),
        Some((c.style.stroke, weight, c.style.dash)),
        theme,
    );
    let mut y = c.rect.y + base * 1.25;
    for line in &c.label {
        text(
            out,
            theme,
            crate::Point::new(c.rect.x + 10.0, y),
            Anchor::Start,
            line,
            c.style.text,
            base,
        );
        y += shape::line_height(line.font().size(base));
    }
}

fn area(out: &mut String, theme: &Theme, a: &crate::Area) {
    if a.path.is_empty() {
        return;
    }
    let _ = write!(
        out,
        r#"<path d="{}" fill="{}""#,
        path_data(&a.path),
        fill_attr(theme, a.fill)
    );
    let opacity = a.opacity.clamp(0.0, 1.0);
    if opacity < 1.0 {
        let _ = write!(out, r#" fill-opacity="{}""#, r(opacity));
    }
    if let Some(pen) = a.stroke
        && pen.paint != Paint::None
    {
        let width = pen.weight.width();
        let _ = write!(
            out,
            r#" stroke="{}" stroke-width="{width}" stroke-linejoin="round"{}"#,
            color(theme, pen.paint),
            dash_attr(pen.dash, width)
        );
    }
    out.push_str("/>");
}

fn edge(out: &mut String, theme: &Theme, e: &Edge) {
    let width = e.pen.weight.width();
    path(
        out,
        &e.path,
        "none".into(),
        Some((e.pen.paint, width, e.pen.dash)),
        theme,
    );
    let ends = [
        (e.end, flatten::end(&e.path)),
        (e.start, flatten::start(&e.path)),
    ];
    for (m, at) in ends {
        let Some((tip, dir)) = at else { continue };
        for part in shape::marker(m, tip, dir, width).parts {
            let fill = match part.fill {
                MarkerFill::None => "none".to_string(),
                MarkerFill::Line => color(theme, e.pen.paint),
                MarkerFill::Background => theme.background.to_string(),
            };
            path(
                out,
                &part.path,
                fill,
                Some((e.pen.paint, width, Dash::Solid)),
                theme,
            );
        }
    }
}

fn edge_text(out: &mut String, theme: &Theme, e: &Edge, base: f32) {
    for label in &e.labels {
        let size = |l: &TextLine| l.font().size(base);
        let total: f32 = label
            .lines
            .iter()
            .map(|l| shape::line_height(size(l)))
            .sum();
        if label.pill {
            let w = label.text_width(base) + 12.0;
            let rect = crate::Rect::centered(label.pos, w, total + 4.0);
            path(
                out,
                &shape::rounded_rect(rect, (total + 4.0) / 2.0),
                theme.background.to_string(),
                Some((Paint::Role(Role::Line), 0.6, Dash::Solid)),
                theme,
            );
        }
        let mut top = label.pos.y - total / 2.0;
        for line in &label.lines {
            let s = size(line);
            let lh = shape::line_height(s);
            let y = top + (lh - s) / 2.0 + s * 0.8;
            text(
                out,
                theme,
                crate::Point::new(label.pos.x, y),
                Anchor::Middle,
                line,
                label.paint,
                base,
            );
            top += lh;
        }
    }
    if let Some((n, at)) = e.step {
        let r = base * 0.75;
        let _ = write!(
            out,
            r#"<circle cx="{}" cy="{}" r="{r}" fill="{}"/>"#,
            at.x, at.y, theme.accent
        );
        text(
            out,
            theme,
            at.offset(0.0, base * 0.3),
            Anchor::Middle,
            &TextLine::new(n.to_string(), Font::Bold),
            Paint::Role(Role::Background),
            base * 0.8,
        );
    }
}

fn node(out: &mut String, theme: &Theme, n: &crate::Node, base: f32) {
    let o = shape::outline(n.shape, n.rect);
    let weight = if n.style.emphasis { 2.2 } else { 1.0 };
    let stroke = Some((n.style.stroke, weight, n.style.dash));
    path(out, &o.body, fill_attr(theme, n.style.fill), stroke, theme);
    for extra in &o.extras {
        path(out, extra, fill_attr(theme, n.style.fill), stroke, theme);
    }
    for detail in &o.details {
        path(out, detail, "none".into(), stroke, theme);
    }
    let layout = shape::text_layout(n, base);
    for (a, b) in &layout.rules {
        let _ = write!(
            out,
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1"/>"#,
            a.x,
            a.y,
            b.x,
            b.y,
            color(theme, n.style.stroke)
        );
    }
    let ink = text_paint(&n.style);
    for placed in &layout.lines {
        let paint = if placed.muted && !matches!(n.style.fill, Paint::Rgb(..)) {
            Paint::Role(Role::Muted)
        } else {
            ink
        };
        text(
            out,
            theme,
            placed.pos,
            placed.anchor,
            &placed.line,
            paint,
            base,
        );
    }
    // Badges along the top edge, from the right.
    let mut right = n.rect.right() - 4.0;
    for badge in &n.badges {
        let w = badge.text.chars().count() as f32 * base * 0.5 + 10.0;
        let h = base * 1.1;
        let rect = crate::Rect::new(right - w, n.rect.y - h / 2.0, w, h);
        path(
            out,
            &shape::rounded_rect(rect, h / 2.0),
            color(theme, badge.paint),
            None,
            theme,
        );
        text(
            out,
            theme,
            crate::Point::new(rect.center().x, rect.y + h * 0.75),
            Anchor::Middle,
            &TextLine::new(badge.text.clone(), Font::Small),
            Paint::Role(Role::Background),
            base,
        );
        right -= w + 4.0;
    }
}

fn legend_block(out: &mut String, theme: &Theme, legend: &crate::Legend, base: f32) {
    path(
        out,
        &shape::rounded_rect(legend.rect, 6.0),
        theme.surface.to_string(),
        Some((Paint::Role(Role::Muted), 0.8, Dash::Solid)),
        theme,
    );
    let row = base * 1.6;
    for (i, entry) in legend.entries.iter().enumerate() {
        let y = legend.rect.y + 8.0 + row * i as f32;
        let swatch = crate::Rect::new(legend.rect.x + 10.0, y + 2.0, row * 1.2, row - 6.0);
        let o = shape::outline(entry.shape, swatch);
        path(
            out,
            &o.body,
            fill_attr(theme, entry.style.fill),
            Some((entry.style.stroke, 1.0, entry.style.dash)),
            theme,
        );
        text(
            out,
            theme,
            crate::Point::new(swatch.right() + 8.0, y + row * 0.65),
            Anchor::Start,
            &TextLine::new(entry.label.clone(), Font::Small),
            Paint::Role(Role::Text),
            base,
        );
    }
}

fn text(
    out: &mut String,
    theme: &Theme,
    pos: crate::Point,
    anchor: Anchor,
    line: &TextLine,
    paint: Paint,
    base: f32,
) {
    let anchor = match anchor {
        Anchor::Start => "start",
        Anchor::Middle => "middle",
        Anchor::End => "end",
    };
    let _ = write!(
        out,
        r#"<text x="{}" y="{}" text-anchor="{anchor}" fill="{}">"#,
        pos.x,
        pos.y,
        color(theme, paint)
    );
    for run in &line.runs {
        let size = run.font.size(base);
        let mut style = format!(r#" font-size="{size}""#);
        if run.font.bold() {
            style.push_str(r#" font-weight="bold""#);
        }
        match run.font {
            Font::Italic => style.push_str(r#" font-style="italic""#),
            Font::Mono => style.push_str(r#" font-family="Menlo, Consolas, monospace""#),
            _ => {}
        }
        let _ = write!(out, "<tspan{style}>{}</tspan>", escape(&run.text));
    }
    out.push_str("</text>");
}

fn path(
    out: &mut String,
    segs: &[Seg],
    fill: String,
    stroke: Option<(Paint, f32, Dash)>,
    theme: &Theme,
) {
    if segs.is_empty() {
        return;
    }
    let _ = write!(out, r#"<path d="{}" fill="{fill}""#, path_data(segs));
    if let Some((paint, width, dash)) = stroke
        && paint != Paint::None
    {
        let _ = write!(
            out,
            r#" stroke="{}" stroke-width="{width}" stroke-linejoin="round" stroke-linecap="round"{}"#,
            color(theme, paint),
            dash_attr(dash, width)
        );
    }
    out.push_str("/>");
}

/// SVG path data for `segs`.
pub fn path_data(segs: &[Seg]) -> String {
    let mut d = String::new();
    for seg in segs {
        let _ = match *seg {
            Seg::Move(p) => write!(d, "M{} {}", r(p.x), r(p.y)),
            Seg::Line(p) => write!(d, "L{} {}", r(p.x), r(p.y)),
            Seg::Cubic(a, b, p) => write!(
                d,
                "C{} {} {} {} {} {}",
                r(a.x),
                r(a.y),
                r(b.x),
                r(b.y),
                r(p.x),
                r(p.y)
            ),
            Seg::Close => write!(d, "Z"),
        };
    }
    d
}

/// Two decimals is plenty, and keeps files small.
fn r(v: f32) -> f32 {
    (v * 100.0).round() / 100.0
}

fn color(theme: &Theme, paint: Paint) -> String {
    theme
        .resolve(paint)
        .map_or_else(|| "none".to_string(), |c| c.to_string())
}

fn fill_attr(theme: &Theme, paint: Paint) -> String {
    color(theme, paint)
}

fn dash_attr(dash: Dash, width: f32) -> String {
    dash.pattern(width)
        .map(|(on, off)| format!(r#" stroke-dasharray="{on} {off}""#))
        .unwrap_or_default()
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn attr(s: &str) -> String {
    escape(s).replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Edge, Marker, Node, Rect, Shape, route};

    fn sample() -> Scene {
        let mut scene = Scene::new(300.0, 120.0);
        let a = Node::new("a", Shape::Person, Rect::new(10.0, 10.0, 100.0, 90.0))
            .with_label("Customer");
        let b = Node::new("b", Shape::Cylinder, Rect::new(190.0, 20.0, 100.0, 70.0))
            .with_label("Ledger <DB>");
        let mut e = Edge::new(route::straight(&a, &b)).between("a", "b");
        e.end = Marker::CrowFoot(crate::Cardinality::OneOrMany);
        scene.edges.push(e);
        scene.nodes.push(a);
        scene.nodes.push(b);
        scene
    }

    #[test]
    fn writes_a_themed_document_without_css() {
        let theme = Theme::dark();
        let svg = to_svg(&sample(), &theme);
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        assert!(!svg.contains("<style") && !svg.contains("@media"));
        assert!(svg.contains(&theme.background.to_string()));
        assert!(svg.contains("Ledger &lt;DB&gt;"), "text is escaped");
        assert!(svg.contains(&theme.text.to_string()));
    }

    #[test]
    fn accessibility_comes_first_and_is_escaped() {
        let mut scene = sample();
        scene.accessibility.title = Some("Pay & ship".into());
        scene.accessibility.description = Some("Customer <-> ledger".into());
        let svg = to_svg(&scene, &Theme::light());
        let body = &svg[svg.find('>').unwrap() + 1..];
        assert!(svg.contains(r#"role="img""#));
        assert!(
            body.starts_with("<title>Pay &amp; ship</title><desc>Customer &lt;-&gt; ledger</desc>")
        );
    }

    #[test]
    fn no_accessibility_writes_no_title() {
        let svg = to_svg(&sample(), &Theme::light());
        assert!(!svg.contains("<title>") && !svg.contains("<desc>"));
    }

    #[test]
    fn areas_fill_under_edges_with_their_opacity() {
        let mut scene = sample();
        let wedge = crate::shape::polygon(&[
            crate::Point::new(0.0, 0.0),
            crate::Point::new(50.0, 0.0),
            crate::Point::new(0.0, 50.0),
        ]);
        let mut area = crate::Area::new(wedge, Paint::Rgb(0x12, 0x34, 0x56));
        area.opacity = 0.5;
        area.stroke = Some(crate::Pen::default());
        scene.areas.push(area);
        let svg = to_svg(&scene, &Theme::light());
        let at = svg
            .find(r##"fill="#123456" fill-opacity="0.5""##)
            .expect("area drawn");
        let first_edge = svg.find("stroke-linecap").expect("edge drawn");
        assert!(at < first_edge, "areas paint under edges");
    }

    #[test]
    fn rotated_text_turns_about_its_anchor() {
        let mut scene = sample();
        let mut t = crate::Text::new(
            crate::Point::new(20.0, 60.0),
            Anchor::Middle,
            TextLine::new("Revenue", Font::Body),
        );
        t.rotate = -90.0;
        t.backed = true;
        scene.texts.push(t);
        let svg = to_svg(&scene, &Theme::light());
        let g = svg
            .find(r#"<g transform="rotate(-90 20 60)">"#)
            .expect("rotated group");
        let close = svg[g..].find("</g>").expect("group closed") + g;
        assert!(svg[g..close].contains("Revenue"), "text inside the group");
        assert!(svg[g..close].contains("<path"), "its backing turns with it");
    }

    #[test]
    fn edge_labels_take_their_own_colour() {
        let mut scene = sample();
        let label = crate::EdgeLabel {
            paint: Paint::Rgb(0xab, 0xcd, 0xef),
            ..crate::EdgeLabel::new(
                crate::Point::new(150.0, 50.0),
                vec![TextLine::new("pays", Font::Small)],
                true,
                &crate::Estimate,
                14.0,
            )
        };
        scene.edges[0].labels.push(label);
        let svg = to_svg(&scene, &Theme::light());
        assert!(svg.contains(r##"fill="#abcdef">"##));
    }
}
