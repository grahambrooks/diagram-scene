//! Every shape, marker and container in one scene, written as SVG and PNG
//! in the light and dark themes: `cargo run --example gallery -- <dir>`.

use diagram_scene::shape::natural_size;
use diagram_scene::{
    Anchor, Badge, Cardinality, Container, ContainerKind, Dash, Edge, EdgeLabel, Estimate, Font,
    Legend, LegendEntry, Marker, Node, Paint, Point, Rect, Role, Scene, Shape, Style, Text,
    TextLine, Theme, route,
};

const SHAPES: [(Shape, &str); 24] = [
    (Shape::Rect, "Rect"),
    (Shape::Rounded, "Rounded"),
    (Shape::Stadium, "Stadium"),
    (Shape::Circle, "Circle"),
    (Shape::Ellipse, "Ellipse"),
    (Shape::Diamond, "Diamond"),
    (Shape::Hexagon, "Hexagon"),
    (Shape::Parallelogram, "Parallelogram"),
    (Shape::Trapezoid, "Trapezoid"),
    (Shape::Cylinder, "Cylinder"),
    (Shape::Document, "Document"),
    (Shape::Subroutine, "Subroutine"),
    (Shape::Person, "Person"),
    (Shape::Actor, "Actor"),
    (Shape::Note, "Note"),
    (Shape::Component, "Component"),
    (Shape::Package, "Package"),
    (Shape::Queue, "Queue"),
    (Shape::Cloud, "Cloud"),
    (Shape::Start, "Start"),
    (Shape::End, "End"),
    (Shape::ForkBar, "Fork"),
    (Shape::Table, "Account"),
    (Shape::Class, "Order"),
];

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let scene = gallery();
    for (name, theme) in [("light", Theme::light()), ("dark", Theme::dark())] {
        let svg = diagram_scene::svg::to_svg(&scene, &theme);
        let path = format!("{dir}/gallery-{name}.svg");
        std::fs::write(&path, &svg).unwrap();
        let mut opt = resvg::usvg::Options::default();
        opt.fontdb_mut().load_system_fonts();
        let tree = resvg::usvg::Tree::from_str(&svg, &opt).unwrap();
        let size = tree.size().to_int_size();
        let mut pixmap =
            resvg::tiny_skia::Pixmap::new(size.width() * 2, size.height() * 2).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(2.0, 2.0),
            &mut pixmap.as_mut(),
        );
        pixmap
            .save_png(format!("{dir}/gallery-{name}.png"))
            .unwrap();
        println!("{path}");
    }
}

fn gallery() -> Scene {
    let base = 14.0;
    let mut scene = Scene::new(980.0, 860.0);
    scene.title = Some(Text {
        pos: Point::new(490.0, 30.0),
        anchor: Anchor::Middle,
        line: TextLine::new("diagram-scene gallery", Font::Title),
        paint: Paint::Role(Role::Text),
        backed: false,
        rotate: 0.0,
    });
    // Shapes in a grid.
    let (cols, cell_w, cell_h, top) = (6, 160.0, 120.0, 60.0);
    for (i, (shape, name)) in SHAPES.iter().enumerate() {
        let mut node = Node::new(format!("s{i}"), *shape, Rect::default()).with_label(name);
        match shape {
            Shape::Table => {
                node.compartments = vec![vec![
                    TextLine::new("id uuid PK", Font::Mono),
                    TextLine::new("owner text", Font::Mono),
                ]];
            }
            Shape::Class => {
                node.stereotype = vec![TextLine::new("«entity»", Font::Small)];
                node.compartments = vec![
                    vec![TextLine::new("- total: Money", Font::Mono)],
                    vec![TextLine::new("+ place()", Font::Mono)],
                ];
            }
            Shape::Cylinder => {
                node.sublabel = vec![TextLine::new("[PostgreSQL]", Font::Small)];
                node.badges = vec![Badge {
                    text: "PII".into(),
                    paint: Paint::Role(Role::Accent),
                }];
            }
            _ => {}
        }
        let (w, h) = natural_size(&node, base, &Estimate);
        let (w, h) = (w.min(cell_w - 4.0), h.min(cell_h - 10.0));
        let c = Point::new(
            20.0 + cell_w * (i % cols) as f32 + cell_w / 2.0,
            top + cell_h * (i / cols) as f32 + cell_h / 2.0,
        );
        node.rect = Rect::centered(c, w, h);
        if *shape == Shape::Rounded {
            node.style.emphasis = true;
            node.style.stroke = Paint::Role(Role::Accent);
        }
        if *shape == Shape::Rect {
            node.style.fill = Paint::Rgb(0xff, 0xd8, 0x66);
        }
        scene.nodes.push(node);
    }
    // Markers, one edge each.
    let markers = [
        (Marker::Arrow, Dash::Solid, "Arrow"),
        (Marker::OpenArrow, Dash::Dashed, "Open"),
        (Marker::HollowTriangle, Dash::Solid, "Inherits"),
        (Marker::FilledDiamond, Dash::Solid, "Composes"),
        (Marker::HollowDiamond, Dash::Dotted, "Aggregates"),
        (Marker::Circle, Dash::Solid, "Circle"),
        (Marker::Cross, Dash::Solid, "Cross"),
        (Marker::CrowFoot(Cardinality::One), Dash::Solid, "One"),
        (
            Marker::CrowFoot(Cardinality::ZeroOrOne),
            Dash::Solid,
            "0..1",
        ),
        (
            Marker::CrowFoot(Cardinality::OneOrMany),
            Dash::Solid,
            "1..*",
        ),
        (
            Marker::CrowFoot(Cardinality::ZeroOrMany),
            Dash::Solid,
            "0..*",
        ),
    ];
    let y0 = 600.0;
    for (i, (m, dash, name)) in markers.iter().enumerate() {
        let x = 30.0 + i as f32 * 86.0;
        let path = route::rounded_polyline(
            &[
                Point::new(x, y0),
                Point::new(x, y0 + 40.0),
                Point::new(x + 60.0, y0 + 40.0),
                Point::new(x + 60.0, y0 + 90.0),
            ],
            8.0,
        );
        let mut e = Edge::new(path);
        e.end = *m;
        e.pen.dash = *dash;
        if i == 0 {
            e.step = Some((1, Point::new(x + 30.0, y0 + 40.0)));
            e.pen.paint = Paint::Role(Role::Accent);
        }
        e.labels.push(EdgeLabel {
            pos: Point::new(x + 30.0, y0 + 110.0),
            lines: vec![TextLine::new(*name, Font::Small)],
            pill: i % 2 == 0,
            width: 0.0,
            paint: Paint::Role(Role::Muted),
        });
        scene.edges.push(e);
    }
    // A routed spline between two shapes.
    let (a, b) = (&scene.nodes[1], &scene.nodes[9]);
    let mut e = Edge::new(route::spline(a.rect, b.rect, false));
    e.labels.push(EdgeLabel {
        pos: route::label_position(&e.path, 0.0).unwrap(),
        lines: vec![TextLine::new("reads/writes", Font::Small)],
        pill: true,
        width: 0.0,
        // A diagram-set label colour.
        paint: Paint::Rgb(0xb8, 0x86, 0x0b),
    });
    scene.edges.push(e);
    // Containers.
    let kinds = [
        (ContainerKind::Boundary, "Boundary"),
        (ContainerKind::DeploymentNode, "Deployment node"),
        (ContainerKind::TrustZone(3), "Restricted zone"),
        (
            ContainerKind::Fragment {
                operator: "alt".into(),
                guards: vec![
                    (0.0, TextLine::new("[ok]", Font::Small)),
                    (790.0, TextLine::new("[else]", Font::Small)),
                ],
            },
            "",
        ),
    ];
    for (i, (kind, label)) in kinds.into_iter().enumerate() {
        let rect = Rect::new(20.0 + i as f32 * 240.0, 740.0, 220.0, 100.0);
        scene.containers.push(Container {
            rect,
            label: if label.is_empty() {
                vec![]
            } else {
                vec![TextLine::new(label, Font::Bold)]
            },
            style: Style::for_container(&kind),
            kind,
        });
    }
    scene.legend = Some(Legend {
        rect: Rect::new(990.0 - 200.0, 560.0, 170.0, 80.0),
        entries: vec![
            LegendEntry {
                shape: Shape::Person,
                style: Style::for_shape(Shape::Person),
                label: "Person".into(),
            },
            LegendEntry {
                shape: Shape::Cylinder,
                style: Style::for_shape(Shape::Cylinder),
                label: "Database".into(),
            },
            LegendEntry {
                shape: Shape::Rounded,
                style: Style::for_shape(Shape::Rounded),
                label: "Container".into(),
            },
        ],
    });
    areas(&mut scene);
    scene.accessibility.title = Some("diagram-scene gallery".into());
    scene.accessibility.description =
        Some("Every shape, marker, container and area the scene model can draw.".into());
    scene.width = 1000.0;
    scene.height = 1010.0;
    scene
}

/// Filled areas: solid pie wedges, translucent overlapping sets, and a
/// rotated axis title.
fn areas(scene: &mut Scene) {
    use diagram_scene::{Area, Pen, Seg, Weight};
    let (cx, cy, r) = (120.0, 930.0, 60.0);
    let mut start = -std::f32::consts::FRAC_PI_2;
    for (i, share) in [0.5_f32, 0.3, 0.2].into_iter().enumerate() {
        let end = start + share * std::f32::consts::TAU;
        let mut path = vec![Seg::Move(Point::new(cx, cy))];
        let steps = (share * 48.0).ceil() as usize;
        for k in 0..=steps {
            let a = start + (end - start) * k as f32 / steps as f32;
            path.push(Seg::Line(Point::new(cx + r * a.cos(), cy + r * a.sin())));
        }
        path.push(Seg::Close);
        let mut wedge = Area::new(path, Paint::Role(Role::Series(i as u8)));
        wedge.stroke = Some(Pen {
            paint: Paint::Role(Role::Background),
            weight: Weight::Thick,
            ..Pen::default()
        });
        scene.areas.push(wedge);
        start = end;
    }
    for (i, x) in [330.0_f32, 400.0].into_iter().enumerate() {
        let mut set = Area::new(
            diagram_scene::shape::ellipse(Point::new(x, 930.0), 60.0, 60.0),
            Paint::Role(Role::Series(i as u8 + 3)),
        );
        set.opacity = 0.35;
        set.stroke = Some(Pen {
            paint: Paint::Role(Role::Series(i as u8 + 3)),
            ..Pen::default()
        });
        scene.areas.push(set);
    }
    let mut axis = Text::new(
        Point::new(540.0, 930.0),
        Anchor::Middle,
        TextLine::new("Rotated -90°", Font::Bold),
    );
    axis.rotate = -90.0;
    scene.texts.push(axis);
}
