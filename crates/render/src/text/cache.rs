use wgpu::*;

use crate::text::*;
use crate::{Cache, Theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphReq {
    pub glyph: u16,
    pub x_fract: Unit,
    pub y_fract: Unit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphStyle {
    pub size: Unit,
    pub weight: Unit,
    pub italic: bool,
    pub hint: bool,
    pub theme: Option<Theme>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct GlyphKey {
    font: FontId,
    glyph: u16,
    style: GlyphStyle,
}

#[derive(Debug)]
struct GlyphData {
    colored: bool,
    view: AtlasView,
}

#[derive(Debug)]
pub struct GlyphCache {
    cache: Cache<GlyphKey, Option<GlyphData>>,
    atlas: Atlas,
    colored_atlas: Atlas,
    rcx: RasterContext,
}

impl GlyphCache {
    pub fn new() -> Self {
        Self {
            cache: Cache::new(),
            atlas: Atlas::new(false),
            colored_atlas: Atlas::new(true),
            rcx: RasterContext::new(),
        }
    }

    pub fn fetch_batch(
        &mut self,
        device: &Device,
        queue: &Queue,
        font_map: &FontMap,
        font_id: FontId,
        style: GlyphStyle,
        reqs: impl Iterator<Item = GlyphReq>,
    ) -> Result<Vec<AtlasPart>, RasterizationError> {
        let mut buf = Vec::new();
        self.fetch_batch_into(device, queue, font_map, font_id, style, &mut buf, reqs)?;
        Ok(buf)
    }

    pub fn fetch_batch_into(
        &mut self,
        device: &Device,
        queue: &Queue,
        font_map: &FontMap,
        font_id: FontId,
        style: GlyphStyle,
        buf: &mut Vec<AtlasPart>,
        reqs: impl Iterator<Item = GlyphReq>,
    ) -> Result<(), RasterizationError> {
        font_map.with(font_id, |font| {
            self.fetch_batch_into_unchecked(device, queue, font, font_id, style, buf, reqs)
        })
    }

    fn fetch_batch_into_unchecked(
        &mut self,
        device: &Device,
        queue: &Queue,
        font: &FontRef,
        font_id: FontId,
        style: GlyphStyle,
        buf: &mut Vec<AtlasPart>,
        reqs: impl Iterator<Item = GlyphReq>,
    ) -> Result<(), RasterizationError> {
        buf.reserve(reqs.size_hint().0);

        let mut scope = self.rcx.open(RasterStyle {
            font: font.clone(),
            size: style.size,
            weight: style.weight,
            italic: style.italic,
            hint: style.hint,
            theme: style.theme,
        });

        for req in reqs {
            let data = self.cache.try_fetch(
                GlyphKey {
                    font: font_id,
                    glyph: req.glyph,
                    style,
                },
                |key| {
                    let image = scope.rasterize(key.glyph, req.x_fract, req.y_fract)?;

                    if image.is_empty() {
                        return Ok(None);
                    }

                    let view = if image.colored {
                        self.colored_atlas.store(device, queue, &image)
                    } else {
                        self.atlas.store(device, queue, &image)
                    };

                    Ok(Some(GlyphData {
                        colored: image.colored,
                        view,
                    }))
                },
            )?;

            if let Some(data) = data {
                let part = if data.colored {
                    self.colored_atlas.read(&data.view)
                } else {
                    self.atlas.read(&data.view)
                };

                buf.push(part);
            }
        }

        Ok(())
    }

    pub fn update(&mut self) {
        for (_, data) in self.cache.sweep() {
            let Some(data) = data else { continue };

            if data.colored {
                self.colored_atlas.remove(data.view);
            } else {
                self.atlas.remove(data.view);
            }
        }

        self.cache.tick();
    }
}
