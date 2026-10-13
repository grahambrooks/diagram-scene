//! What each [`Role`] looks like, for painters that don't bring their own
//! palette (the SVG writer). An editor maps roles onto its own theme.

use crate::{Paint, Role, Style};

/// An sRGB colour, written as `#rrggbb`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color(pub u8, pub u8, pub u8);

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

impl Color {
    /// Relative luminance, 0 (black) to 1 (white).
    pub fn luminance(self) -> f32 {
        let lin = |c: u8| {
            let c = c as f32 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(self.0) + 0.7152 * lin(self.1) + 0.0722 * lin(self.2)
    }

    /// The colour a fraction `t` of the way to `other`.
    pub fn mix(self, other: Color, t: f32) -> Color {
        let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Color(m(self.0, other.0), m(self.1, other.1), m(self.2, other.2))
    }
}

/// Colours for every role, and the font to name in SVG.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub font_family: String,
    pub background: Color,
    pub surface: Color,
    pub text: Color,
    pub muted: Color,
    pub line: Color,
    pub accent: Color,
    pub highlight: Color,
    pub person: Color,
    pub system: Color,
    pub external: Color,
    pub container: Color,
    pub component: Color,
    pub database: Color,
    /// Trust zones from public up; the last repeats for higher levels.
    pub zones: Vec<Color>,
    /// Cycled for [`Role::Series`].
    pub series: Vec<Color>,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::light()
    }
}

impl Theme {
    pub fn light() -> Theme {
        let bg = Color(0xff, 0xff, 0xff);
        let tint = |c: Color| bg.mix(c, 0.16);
        Theme {
            font_family: "Inter, 'Helvetica Neue', Helvetica, Arial, sans-serif".into(),
            background: bg,
            surface: Color(0xf6, 0xf8, 0xfa),
            text: Color(0x1f, 0x23, 0x28),
            muted: Color(0x65, 0x6d, 0x76),
            line: Color(0x57, 0x60, 0x6a),
            accent: Color(0x09, 0x69, 0xda),
            highlight: Color(0xff, 0xf8, 0xc5),
            person: tint(Color(0x08, 0x42, 0x7b)),
            system: tint(Color(0x11, 0x68, 0xbd)),
            external: tint(Color(0x8b, 0x94, 0x9e)),
            container: tint(Color(0x43, 0x8d, 0xd5)),
            component: tint(Color(0x85, 0xbb, 0xf0)),
            database: tint(Color(0x1a, 0x7f, 0x37)),
            zones: vec![
                bg.mix(Color(0x1a, 0x7f, 0x37), 0.06),
                bg.mix(Color(0x09, 0x69, 0xda), 0.06),
                bg.mix(Color(0x9a, 0x67, 0x00), 0.07),
                bg.mix(Color(0xcf, 0x22, 0x2e), 0.07),
            ],
            series: vec![
                Color(0x09, 0x69, 0xda),
                Color(0x1a, 0x7f, 0x37),
                Color(0x82, 0x50, 0xdf),
                Color(0xbf, 0x39, 0x89),
                Color(0x9a, 0x67, 0x00),
                Color(0xcf, 0x22, 0x2e),
            ],
        }
    }

    pub fn dark() -> Theme {
        let bg = Color(0x1c, 0x1e, 0x24);
        let tint = |c: Color| bg.mix(c, 0.28);
        Theme {
            font_family: "Inter, 'Helvetica Neue', Helvetica, Arial, sans-serif".into(),
            background: bg,
            surface: Color(0x26, 0x29, 0x31),
            text: Color(0xe6, 0xe6, 0xe6),
            muted: Color(0x9d, 0xa5, 0xb4),
            line: Color(0x8b, 0x94, 0x9e),
            accent: Color(0x58, 0xa6, 0xff),
            highlight: Color(0x3b, 0x34, 0x1c),
            person: tint(Color(0x58, 0xa6, 0xff)),
            system: tint(Color(0x38, 0x8b, 0xfd)),
            external: tint(Color(0x8b, 0x94, 0x9e)),
            container: tint(Color(0x79, 0xc0, 0xff)),
            component: tint(Color(0xa5, 0xd6, 0xff)),
            database: tint(Color(0x3f, 0xb9, 0x50)),
            zones: vec![
                bg.mix(Color(0x3f, 0xb9, 0x50), 0.08),
                bg.mix(Color(0x58, 0xa6, 0xff), 0.08),
                bg.mix(Color(0xd2, 0x99, 0x22), 0.09),
                bg.mix(Color(0xf8, 0x51, 0x49), 0.09),
            ],
            series: vec![
                Color(0x58, 0xa6, 0xff),
                Color(0x3f, 0xb9, 0x50),
                Color(0xbc, 0x8c, 0xff),
                Color(0xff, 0x7b, 0x72),
                Color(0xd2, 0x99, 0x22),
                Color(0x39, 0xc5, 0xcf),
            ],
        }
    }

    /// Whether text on the background should be light.
    pub fn is_dark(&self) -> bool {
        self.background.luminance() < 0.4
    }

    /// The colour of a role.
    pub fn role(&self, role: Role) -> Color {
        let cycle = |list: &[Color], i: usize| {
            if list.is_empty() {
                self.accent
            } else {
                list[i.min(list.len() - 1)]
            }
        };
        match role {
            Role::Background => self.background,
            Role::Surface => self.surface,
            Role::Text => self.text,
            Role::Muted => self.muted,
            Role::Line => self.line,
            Role::Accent => self.accent,
            Role::Highlight => self.highlight,
            Role::Person => self.person,
            Role::System => self.system,
            Role::External => self.external,
            Role::Container => self.container,
            Role::Component => self.component,
            Role::Database => self.database,
            Role::Zone(level) => cycle(&self.zones, level as usize),
            Role::Series(i) => {
                if self.series.is_empty() {
                    self.accent
                } else {
                    self.series[i as usize % self.series.len()]
                }
            }
        }
    }

    /// The colour of a paint, or `None` for no paint.
    pub fn resolve(&self, paint: Paint) -> Option<Color> {
        match paint {
            Paint::None => None,
            Paint::Role(r) => Some(self.role(r)),
            Paint::Rgb(r, g, b) => Some(Color(r, g, b)),
        }
    }
}

/// The paint for text on a node with `style`: its own text paint, unless
/// the diagram set the fill itself, when black or white is chosen to read
/// on it.
pub fn text_paint(style: &Style) -> Paint {
    match style.fill {
        Paint::Rgb(r, g, b) => {
            if Color(r, g, b).luminance() > 0.35 {
                Paint::Rgb(0x1f, 0x23, 0x28)
            } else {
                Paint::Rgb(0xf6, 0xf8, 0xfa)
            }
        }
        _ => style.text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_resolve_and_text_reads_on_custom_fills() {
        let dark = Theme::dark();
        assert!(dark.is_dark() && !Theme::light().is_dark());
        assert_eq!(dark.resolve(Paint::Role(Role::Text)), Some(dark.text));
        assert_eq!(dark.resolve(Paint::None), None);
        assert_eq!(dark.role(Role::Zone(9)), *dark.zones.last().unwrap());
        let mut style = Style::for_shape(crate::Shape::Rounded);
        style.fill = Paint::Rgb(0xff, 0xee, 0x88);
        assert_eq!(text_paint(&style), Paint::Rgb(0x1f, 0x23, 0x28));
        style.fill = Paint::Rgb(0x10, 0x20, 0x40);
        assert_eq!(text_paint(&style), Paint::Rgb(0xf6, 0xf8, 0xfa));
    }
}
