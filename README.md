# diagram-scene

A laid-out software diagram, as data. Layout engines produce a `Scene`;
painters draw it.

- [`mmd`](https://github.com/grahambrooks/mmd) lays out Mermaid (flowcharts,
  git graphs, C4 context).
- [`puml`](https://github.com/grahambrooks/puml) lays out PlantUML (sequence,
  class, activity, state).
- [`forge`](https://github.com/grahambrooks/forge) lays out its architecture
  views.
- The [emx](https://github.com/grahambrooks/emx) editor paints scenes
  natively in its own theme; `svg::to_svg` writes them as SVG.

## The model

A scene is semantic rather than a list of primitives:

- **Nodes** with a `Shape`: rect, rounded, stadium, circle, ellipse, diamond,
  hexagon, parallelogram, trapezoid, cylinder, document, subroutine, person,
  actor, note, component, package, queue, cloud, start/end dots, fork bars,
  and tables and classes with compartments. They have a stereotype, a label,
  a sublabel, and badges.
- **Containers** drawn under the nodes: groups, subgraphs, C4 boundaries,
  deployment nodes, trust zones, swimlanes, sequence fragments with their
  guards, lifelines.
- **Edges** with their paths already routed, a pen (solid, dashed, dotted;
  thin to heavy), a marker at each end (arrow, open arrow, UML hollow
  triangle and diamonds, circle, cross, crow's feet for each cardinality),
  labels (optionally on a pill) and step numbers.
- **Areas**: filled regions with no label of their own — pie wedges, bars,
  ribbons, set circles — with an opacity (below 1, what is underneath shows
  through) and an optional outline. Drawn over containers, under edges.
- Free text, which may be rotated about its anchor, a title and a legend.
- **Accessibility**: an accessible name and description that painters carry
  where they can (SVG `<title>` and `<desc>`, with `role="img"`) and never
  draw.

Edge labels carry their own colour (muted unless the diagram set one).
`Text::new` and `Text::default()` exist so new fields need not break
callers; prefer them to struct literals.

Colours are `Role`s (background, surface, text, muted, line, accent, person,
system, database, trust zones, series…) that a theme resolves, so a scene is
laid out once and painted in any theme. Only colours a diagram's source sets
itself are concrete (`Paint::Rgb`).

## Shared drawing rules

Every painter draws a scene the same way because the rules live here:

- `shape::outline`: each shape's paths.
- `shape::text_layout`: where a node's text goes.
- `shape::natural_size`: how big a node must be for its text.
- `shape::marker`: the marker at an edge's end.
- `route`: clipping to outlines, plus straight, orthogonal (rounded) and
  spline routes and label placement, for layout engines.
- `flatten`: polylines and triangulation, for painters that only draw
  straight segments or convex fills.

## Features

- `svg` (default): `svg::to_svg(&scene, &Theme)`, with presentation
  attributes and no CSS, in `Theme::light()` or `Theme::dark()`.
- `measure`: `measure::FontMeasure` measures text by shaping it with the
  caller's font, so boxes fit the font they'll be drawn in.

```console
$ cargo run --example gallery -- /tmp   # every shape and marker, light and dark
```
