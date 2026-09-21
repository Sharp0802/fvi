use std::fmt::Debug;
use swash::Usability;
use swash::scale::{Render, ScaleContext, Scaler, Source, StrikeWith};
use swash::zeno::Vector;

use super::SwashImage;
use crate::Theme;
use crate::text::{FontRef, Image, RasterizationError, Unit};

#[derive(Clone, Debug)]
pub struct RasterStyle<'a> {
    pub font: FontRef<'a>,
    pub size: Unit,
    pub weight: Unit,
    pub italic: bool,
    pub hint: bool,
    pub theme: Option<Theme>,
}

pub struct RasterContext {
    context: ScaleContext,
    buffer: SwashImage,
}

impl RasterContext {
    pub fn new() -> Self {
        Self {
            context: ScaleContext::new(),
            buffer: SwashImage::new(),
        }
    }

    pub fn open<'a>(&'a mut self, style: RasterStyle<'a>) -> RasterScope<'a> {
        RasterScope::new(self, style)
    }
}

impl Debug for RasterContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RasterContext").finish_non_exhaustive()
    }
}

pub struct RasterScope<'a> {
    palette: Option<u16>,
    scaler: Scaler<'a>,
    buffer: &'a mut SwashImage,
}

impl<'a> RasterScope<'a> {
    fn new(rcx: &'a mut RasterContext, style: RasterStyle<'a>) -> Self {
        Self {
            palette: query_palette(&style.font, style.theme),
            scaler: rcx
                .context
                .builder(style.font)
                .size(style.size.into())
                .variations(&[("wght", style.weight.into()), ("ital", style.italic.into())])
                .hint(style.hint)
                .build(),
            buffer: &mut rcx.buffer,
        }
    }

    pub fn rasterize(
        &mut self,
        glyph: u16,
        x_fract: f32,
        y_fract: f32,
    ) -> Result<Image<'_>, RasterizationError> {
        {
            #![expect(clippy::float_cmp, reason = ".fract() always returns exact result")]
            debug_assert_eq!(x_fract.fract(), x_fract);
            debug_assert_eq!(y_fract.fract(), y_fract);
        }

        let sources_base: [Source; 4] = [
            Source::ColorOutline(self.palette.unwrap_or(0)),
            Source::Outline,
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Bitmap(StrikeWith::BestFit),
        ];
        let sources = if self.palette.is_some() {
            &sources_base
        } else {
            &sources_base[1..]
        };

        self.buffer.clear();

        if !Render::new(sources)
            .offset(Vector::new(x_fract, y_fract))
            .render_into(&mut self.scaler, glyph, self.buffer)
        {
            return Err(RasterizationError);
        }

        Ok(Image::from(self.buffer))
    }
}

fn query_palette(font: &FontRef, theme: Option<Theme>) -> Option<u16> {
    let font: &swash::FontRef = font.as_ref();

    let mut best = None;
    for palette in font.color_palettes() {
        match (palette.usability(), theme) {
            (_, None)
            | (Some(Usability::Both), _)
            | (Some(Usability::Light), Some(Theme::Light))
            | (Some(Usability::Dark), Some(Theme::Dark)) => return Some(palette.index()),
            (None, Some(_)) => best = Some(palette.index()),
            _ => continue,
        }
    }

    best
}
