//! A laid-out software diagram, as data.
//!
//! Layout engines (mmd for Mermaid, puml for PlantUML, forge for its
//! architecture models) produce a [`Scene`]: nodes with their shapes and
//! rectangles, containers drawn under them, and edges whose paths are
//! already routed. Painters draw it: [`svg::to_svg`] for files, and an
//! editor's own drawing layer for a native view.
//!
//! The model is semantic rather than a list of primitives. A node is "a
//! cylinder labelled *Ledger DB*", not four paths, so every painter draws
//! it the same way, in its own style. Colours are [`Role`]s, which a theme
//! resolves, so a scene is laid out once and painted in any theme; only
//! colours a diagram's source sets itself are concrete ([`Paint::Rgb`]).
//!
//! The shared drawing rules live here too: a shape's outline
//! ([`shape::outline`]), where its text goes ([`shape::text_layout`]), the
//! marker at an edge's end ([`shape::marker`]), and edge routing
//! ([`route`]). Painters only fill, stroke and place text.

pub mod flatten;
pub mod geom;
#[cfg(feature = "measure")]
pub mod measure;
pub mod route;
pub mod shape;
#[cfg(feature = "svg")]
pub mod svg;
pub mod theme;

pub use geom::{Point, Rect};
pub use theme::Theme;

/// A laid-out diagram.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scene {
    /// The drawing's extent; everything lies within `(0, 0)..(width, height)`.
    pub width: f32,
    pub height: f32,
    /// Body text size in px; [`Font`]s scale from it.
    pub font_size: f32,
    /// Drawn centred at the top, in the space the layout left for it.
    pub title: Option<Text>,
    /// What assistive technology reads: the accessible name and
    /// description, never drawn.
    pub accessibility: Accessibility,
    /// Drawn first, outermost first (a container precedes those inside it).
    pub containers: Vec<Container>,
    /// Filled regions — wedges, bars, ribbons, set circles — drawn over the
    /// containers and under the edges, in order.
    pub areas: Vec<Area>,
    /// Drawn over the areas.
    pub edges: Vec<Edge>,
    /// Drawn over the edges.
    pub nodes: Vec<Node>,
    /// Free text: axis labels, commit ids, captions.
    pub texts: Vec<Text>,
    pub legend: Option<Legend>,
}

impl Scene {
    /// An empty scene of the given size, with 14 px body text.
    pub fn new(width: f32, height: f32) -> Self {
        Scene {
            width,
            height,
            font_size: 14.0,
            ..Scene::default()
        }
    }

    /// Every piece of text in the scene, in drawing order. For tests and
    /// text fallbacks ("does every label appear?").
    pub fn texts(&self) -> Vec<String> {
        fn lines(out: &mut Vec<String>, lines: &[TextLine]) {
            out.extend(lines.iter().map(TextLine::plain));
        }
        let mut out = Vec::new();
        if let Some(t) = &self.title {
            out.push(t.line.plain());
        }
        for c in &self.containers {
            lines(&mut out, &c.label);
            if let ContainerKind::Fragment { guards, .. } = &c.kind {
                out.extend(guards.iter().map(|(_, g)| g.plain()));
            }
        }
        for e in &self.edges {
            for l in &e.labels {
                lines(&mut out, &l.lines);
            }
        }
        for n in &self.nodes {
            lines(&mut out, &n.stereotype);
            lines(&mut out, &n.label);
            lines(&mut out, &n.sublabel);
            for c in &n.compartments {
                lines(&mut out, c);
            }
            out.extend(n.badges.iter().map(|b| b.text.clone()));
        }
        out.extend(self.texts.iter().map(|t| t.line.plain()));
        if let Some(l) = &self.legend {
            out.extend(l.entries.iter().map(|e| e.label.clone()));
        }
        out
    }

    pub fn bounds(&self) -> Rect {
        Rect::new(0.0, 0.0, self.width, self.height)
    }

    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }
}

/// A diagram's accessible name and description. A painter that can carry
/// them (SVG's `<title>` and `<desc>`) does; none draws them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Accessibility {
    pub title: Option<String>,
    pub description: Option<String>,
}

/// A filled region: a pie wedge, a bar, a sankey ribbon, a set in a Venn
/// diagram. Unlike a [`Node`] it has no label and any outline; unlike an
/// [`Edge`] it is filled. Text that belongs to it goes in [`Scene::texts`].
#[derive(Debug, Clone, PartialEq)]
pub struct Area {
    /// A closed outline (or several, for holes: filled non-zero).
    pub path: Vec<Seg>,
    pub fill: Paint,
    /// 0 (clear) to 1 (solid): below 1, what is underneath shows through.
    pub opacity: f32,
    /// An outline drawn over the fill, if any.
    pub stroke: Option<Pen>,
}

impl Area {
    /// A solid, unoutlined region.
    pub fn new(path: Vec<Seg>, fill: Paint) -> Self {
        Area {
            path,
            fill,
            opacity: 1.0,
            stroke: None,
        }
    }
}

/// A box, circle, person, table…: one element of the diagram.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Unique within the scene; edges refer to it.
    pub id: String,
    pub rect: Rect,
    pub shape: Shape,
    /// Above the label, small: `«interface»`, `[Container]`.
    pub stereotype: Vec<TextLine>,
    /// The name, centred (or in the header, for tables and classes).
    pub label: Vec<TextLine>,
    /// Below the label, muted: technology, description.
    pub sublabel: Vec<TextLine>,
    /// For [`Shape::Class`] and [`Shape::Table`]: the sections under the
    /// header (fields, methods; columns), separated by rules.
    pub compartments: Vec<Vec<TextLine>>,
    /// Small tags pinned to the top-right corner (data classes, counts).
    pub badges: Vec<Badge>,
    pub style: Style,
}

impl Node {
    /// A node with the default style for its shape.
    pub fn new(id: impl Into<String>, shape: Shape, rect: Rect) -> Self {
        Node {
            id: id.into(),
            rect,
            shape,
            stereotype: Vec::new(),
            label: Vec::new(),
            sublabel: Vec::new(),
            compartments: Vec::new(),
            badges: Vec::new(),
            style: Style::for_shape(shape),
        }
    }

    pub fn with_label(mut self, text: &str) -> Self {
        self.label = TextLine::lines(text, Font::Bold);
        self
    }
}

/// The outline a node is drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shape {
    Rect,
    /// Rounded corners: the default box.
    Rounded,
    /// Fully rounded ends (a pill).
    Stadium,
    Circle,
    Ellipse,
    Diamond,
    Hexagon,
    /// Slanted sides (input/output).
    Parallelogram,
    Trapezoid,
    /// A database: a cylinder seen from the side.
    Cylinder,
    /// A page with a wavy bottom edge.
    Document,
    /// A box with doubled sides (a predefined process).
    Subroutine,
    /// A head over rounded shoulders, label in the body (C4 person).
    Person,
    /// A stick figure, label below (UML actor).
    Actor,
    /// A page with a folded corner.
    Note,
    /// A box with the UML component tabs on its left edge.
    Component,
    /// A folder: a box with a tab on its top-left.
    Package,
    /// A horizontal cylinder (a message queue).
    Queue,
    Cloud,
    /// A filled dot (an initial state).
    Start,
    /// A ringed dot (a final state).
    End,
    /// A thick bar (a fork or join).
    ForkBar,
    /// A header row over compartments of rows (an ER entity).
    Table,
    /// A header (stereotype, name) over field and method compartments.
    Class,
}

/// Something drawn under nodes, around the ones it groups.
#[derive(Debug, Clone, PartialEq)]
pub struct Container {
    pub rect: Rect,
    pub kind: ContainerKind,
    /// Drawn in the top-left corner (a tab, for fragments).
    pub label: Vec<TextLine>,
    pub style: Style,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContainerKind {
    /// A plain grouping box.
    Group,
    /// A Mermaid subgraph.
    Subgraph,
    /// A C4 boundary (system, enterprise): dashed.
    Boundary,
    /// A deployment node (a server, a cluster): nested boxes.
    DeploymentNode,
    /// A trust zone; higher levels are more restricted (0 = public).
    TrustZone(u8),
    /// A lane: a branch in a git graph, a swimlane in an activity.
    Swimlane,
    /// A sequence-diagram combined fragment (`alt`, `loop`, `opt`…), with
    /// the y of each dashed divider and its guard.
    Fragment {
        operator: String,
        guards: Vec<(f32, TextLine)>,
    },
    /// A sequence-diagram lifeline: a dashed vertical line down the
    /// middle of `rect`.
    Lifeline,
}

/// A connection, its path already routed.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    /// The node ids it joins, when it joins nodes.
    pub from: Option<String>,
    pub to: Option<String>,
    /// From the start to the end, ending where the markers' tips go.
    pub path: Vec<Seg>,
    pub pen: Pen,
    pub start: Marker,
    pub end: Marker,
    pub labels: Vec<EdgeLabel>,
    /// A numbered step (dynamic views), drawn as a circled number at the
    /// given point.
    pub step: Option<(u32, Point)>,
}

impl Edge {
    /// A plain arrow along `path`.
    pub fn new(path: Vec<Seg>) -> Self {
        Edge {
            from: None,
            to: None,
            path,
            pen: Pen::default(),
            start: Marker::None,
            end: Marker::Arrow,
            labels: Vec::new(),
            step: None,
        }
    }

    pub fn between(mut self, from: &str, to: &str) -> Self {
        self.from = Some(from.into());
        self.to = Some(to.into());
        self
    }
}

/// One step of a path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Seg {
    Move(Point),
    Line(Point),
    /// A cubic Bézier: two control points, then the end.
    Cubic(Point, Point, Point),
    Close,
}

/// How an edge's line is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pen {
    pub dash: Dash,
    pub weight: Weight,
    pub paint: Paint,
}

impl Default for Pen {
    fn default() -> Self {
        Pen {
            dash: Dash::Solid,
            weight: Weight::Normal,
            paint: Paint::Role(Role::Line),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dash {
    Solid,
    Dashed,
    Dotted,
}

impl Dash {
    /// (on, off) lengths for a line `width` wide, or `None` for solid.
    pub fn pattern(self, width: f32) -> Option<(f32, f32)> {
        match self {
            Dash::Solid => None,
            Dash::Dashed => Some((6.0 * width.max(1.0), 4.0 * width.max(1.0))),
            Dash::Dotted => Some((width.max(1.0), 3.0 * width.max(1.0))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weight {
    Thin,
    Normal,
    Thick,
    /// A git graph's branch lines.
    Heavy,
}

impl Weight {
    /// The stroke width in px.
    pub fn width(self) -> f32 {
        match self {
            Weight::Thin => 1.0,
            Weight::Normal => 1.4,
            Weight::Thick => 2.6,
            Weight::Heavy => 5.0,
        }
    }
}

/// What's drawn at an end of an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Marker {
    None,
    /// A filled triangle.
    Arrow,
    /// Two strokes, open (`->>`, dependencies).
    OpenArrow,
    /// Inheritance.
    HollowTriangle,
    /// Composition.
    FilledDiamond,
    /// Aggregation.
    HollowDiamond,
    /// A small open circle (`--o`).
    Circle,
    /// A cross (`--x`, a lost message).
    Cross,
    /// Entity-relationship cardinality.
    CrowFoot(Cardinality),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cardinality {
    /// Exactly one: two bars.
    One,
    /// A circle and a bar.
    ZeroOrOne,
    /// A crow's foot.
    Many,
    /// A bar and a crow's foot.
    OneOrMany,
    /// A circle and a crow's foot.
    ZeroOrMany,
}

/// Text along an edge, centred on `pos`.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeLabel {
    pub pos: Point,
    pub lines: Vec<TextLine>,
    /// Drawn on a rounded backing so it reads over lines.
    pub pill: bool,
    /// The widest line's width, as the layout measured it (for the pill);
    /// 0 to let the painter estimate.
    pub width: f32,
    /// The text's colour; [`Role::Muted`] unless the diagram set one.
    pub paint: Paint,
}

impl EdgeLabel {
    /// A label at `pos`, its width measured.
    pub fn new(
        pos: Point,
        lines: Vec<TextLine>,
        pill: bool,
        measure: &dyn Measure,
        base: f32,
    ) -> Self {
        let width = lines
            .iter()
            .map(|l| line_width(measure, l, base))
            .fold(0.0_f32, f32::max);
        EdgeLabel {
            pos,
            lines,
            pill,
            width,
            paint: Paint::Role(Role::Muted),
        }
    }

    /// The width to draw the pill for, estimating if unmeasured.
    pub fn text_width(&self, base: f32) -> f32 {
        if self.width > 0.0 {
            self.width
        } else {
            self.lines
                .iter()
                .map(|l| line_width(&Estimate, l, base))
                .fold(0.0_f32, f32::max)
        }
    }
}

/// How a node or container is filled and outlined.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    pub fill: Paint,
    pub stroke: Paint,
    /// The text colour, for fills that need their own.
    pub text: Paint,
    pub dash: Dash,
    /// Drawn heavier: the focus of a view, a highlighted step.
    pub emphasis: bool,
}

impl Style {
    /// The default look of a shape.
    pub fn for_shape(shape: Shape) -> Style {
        let plain = Style {
            fill: Paint::Role(Role::Surface),
            stroke: Paint::Role(Role::Line),
            text: Paint::Role(Role::Text),
            dash: Dash::Solid,
            emphasis: false,
        };
        match shape {
            Shape::Start | Shape::End | Shape::ForkBar => Style {
                fill: Paint::Role(Role::Text),
                stroke: Paint::Role(Role::Text),
                ..plain
            },
            Shape::Note => Style {
                fill: Paint::Role(Role::Highlight),
                ..plain
            },
            Shape::Cylinder => Style {
                fill: Paint::Role(Role::Database),
                ..plain
            },
            Shape::Person | Shape::Actor => Style {
                fill: Paint::Role(Role::Person),
                ..plain
            },
            _ => plain,
        }
    }

    /// A container's look.
    pub fn for_container(kind: &ContainerKind) -> Style {
        let fill = match kind {
            ContainerKind::TrustZone(level) => Paint::Role(Role::Zone(*level)),
            ContainerKind::Lifeline | ContainerKind::Fragment { .. } => Paint::None,
            ContainerKind::Swimlane => Paint::None,
            _ => Paint::Role(Role::Background),
        };
        let dash = match kind {
            ContainerKind::Boundary
            | ContainerKind::DeploymentNode
            | ContainerKind::TrustZone(_)
            | ContainerKind::Lifeline => Dash::Dashed,
            _ => Dash::Solid,
        };
        Style {
            fill,
            stroke: Paint::Role(Role::Muted),
            text: Paint::Role(Role::Muted),
            dash,
            emphasis: false,
        }
    }
}

/// A colour: a theme's, by role, or the diagram's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Paint {
    None,
    Role(Role),
    Rgb(u8, u8, u8),
}

/// What a colour is for. A theme says what each looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// The page behind the diagram.
    Background,
    /// A node's fill: a little off the background.
    Surface,
    Text,
    /// Secondary text: technology, descriptions, container labels.
    Muted,
    /// Edges and outlines.
    Line,
    /// The focus: a selected element, an emphasised edge.
    Accent,
    /// Notes and highlights.
    Highlight,
    /// C4 and architecture kinds.
    Person,
    System,
    External,
    Container,
    Component,
    Database,
    /// Trust zones, from 0 (public) up to more restricted.
    Zone(u8),
    /// Distinct colours for series: branches, lanes. Cycled by index.
    Series(u8),
}

/// A line of text, in runs of one font each.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextLine {
    pub runs: Vec<Run>,
}

impl TextLine {
    pub fn new(text: impl Into<String>, font: Font) -> Self {
        TextLine {
            runs: vec![Run {
                text: text.into(),
                font,
            }],
        }
    }

    /// One line per line of `text`.
    pub fn lines(text: &str, font: Font) -> Vec<TextLine> {
        text.lines().map(|l| TextLine::new(l, font)).collect()
    }

    pub fn plain(&self) -> String {
        self.runs.iter().map(|r| r.text.as_str()).collect()
    }

    /// The tallest font in the line.
    pub fn font(&self) -> Font {
        self.runs
            .iter()
            .map(|r| r.font)
            .max_by(|a, b| a.scale().total_cmp(&b.scale()))
            .unwrap_or(Font::Body)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub text: String,
    pub font: Font,
}

/// A text style, relative to the scene's body size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Font {
    Body,
    Bold,
    Italic,
    Mono,
    /// Technology, stereotypes, badges, edge labels.
    Small,
    Title,
}

impl Font {
    /// Size relative to the body size.
    pub fn scale(self) -> f32 {
        match self {
            Font::Small => 0.8,
            Font::Title => 1.3,
            _ => 1.0,
        }
    }

    pub fn bold(self) -> bool {
        matches!(self, Font::Bold | Font::Title)
    }

    /// The size in px at body size `base`.
    pub fn size(self, base: f32) -> f32 {
        base * self.scale()
    }
}

/// Free-standing text.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub pos: Point,
    pub anchor: Anchor,
    pub line: TextLine,
    pub paint: Paint,
    /// Drawn on the background colour, so it reads where a line crosses.
    pub backed: bool,
    /// Degrees clockwise about `pos`: 0 is level, -90 reads bottom to top
    /// (a y-axis title).
    pub rotate: f32,
}

impl Text {
    /// Level, unbacked text in the text colour.
    pub fn new(pos: Point, anchor: Anchor, line: TextLine) -> Self {
        Text {
            pos,
            anchor,
            line,
            ..Text::default()
        }
    }
}

impl Default for Text {
    fn default() -> Self {
        Text {
            pos: Point::new(0.0, 0.0),
            anchor: Anchor::Start,
            line: TextLine::default(),
            paint: Paint::Role(Role::Text),
            backed: false,
            rotate: 0.0,
        }
    }
}

/// Where `pos` is on a line of text: its baseline's start, middle or end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Anchor {
    Start,
    Middle,
    End,
}

/// A small tag on a node's corner.
#[derive(Debug, Clone, PartialEq)]
pub struct Badge {
    pub text: String,
    pub paint: Paint,
}

/// A key to the shapes and colours used, in the rectangle the layout left
/// for it.
#[derive(Debug, Clone, PartialEq)]
pub struct Legend {
    pub rect: Rect,
    pub entries: Vec<LegendEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LegendEntry {
    pub shape: Shape,
    pub style: Style,
    pub label: String,
}

/// Measures text as the eventual painter will draw it, so boxes fit their
/// labels.
pub trait Measure {
    /// Advance width of `text` at `size` px.
    fn width(&self, text: &str, size: f32, bold: bool) -> f32;

    /// Baseline-to-baseline distance at `size` px.
    fn line_height(&self, size: f32) -> f32 {
        size * 1.35
    }

    /// Distance from the top of a line box down to its baseline.
    fn ascent(&self, size: f32) -> f32 {
        size * 0.8
    }

    /// Advance width of `text` in the monospace font ([`Font::Mono`]).
    /// Most monospace faces advance 0.6 em per character.
    fn mono_width(&self, text: &str, size: f32) -> f32 {
        text.chars().count() as f32 * size * 0.6
    }
}

/// A rough measure for when no font is at hand: an average advance per
/// character. Good enough for tests and a CLI without fonts.
pub struct Estimate;

impl Measure for Estimate {
    fn width(&self, text: &str, size: f32, bold: bool) -> f32 {
        let per = if bold { 0.58 } else { 0.54 };
        text.chars().count() as f32 * size * per
    }
}

/// The width of a [`TextLine`] at body size `base`.
pub fn line_width(measure: &dyn Measure, line: &TextLine, base: f32) -> f32 {
    line.runs
        .iter()
        .map(|r| match r.font {
            Font::Mono => measure.mono_width(&r.text, r.font.size(base)),
            font => measure.width(&r.text, font.size(base), font.bold()),
        })
        .sum()
}
