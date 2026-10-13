//! Measuring text by shaping it with a font the caller supplies, so a
//! layout sizes boxes for the font they'll be drawn in.

use rustybuzz::{Face, UnicodeBuffer};

/// The font parser, for finding faces in a collection.
pub use rustybuzz::ttf_parser;

use crate::Measure;

/// Measures with a regular and a bold face.
pub struct FontMeasure<'a> {
    regular: Face<'a>,
    bold: Face<'a>,
}

impl<'a> FontMeasure<'a> {
    /// Faces from font data (a `.ttf`, `.otf` or a face of a `.ttc`, by
    /// index). `None` if either can't be read.
    pub fn new(regular: (&'a [u8], u32), bold: (&'a [u8], u32)) -> Option<Self> {
        Some(FontMeasure {
            regular: Face::from_slice(regular.0, regular.1)?,
            bold: Face::from_slice(bold.0, bold.1)?,
        })
    }
}

impl Measure for FontMeasure<'_> {
    fn width(&self, text: &str, size: f32, bold: bool) -> f32 {
        let face = if bold { &self.bold } else { &self.regular };
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        let shaped = rustybuzz::shape(face, &[], buffer);
        let advance: i32 = shaped.glyph_positions().iter().map(|g| g.x_advance).sum();
        advance as f32 * size / face.units_per_em() as f32
    }
}
