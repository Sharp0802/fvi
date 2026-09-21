use wgpu::*;

use crate::text::*;
use crate::{Cache, Theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphStyle {
    pub size: Unit,
    pub weight: Unit,
    pub italic: bool,
    pub hint: bool,
    pub x_fract: Unit,
    pub y_fract: Unit,
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
    cache: Cache<GlyphKey, GlyphData>,
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
        gids: impl Iterator<Item = u16>,
    ) -> Result<Vec<AtlasPart>, RasterizationError> {
        let mut buf = Vec::new();
        self.fetch_batch_into(device, queue, font_map, font_id, style, &mut buf, gids)?;
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
        gids: impl Iterator<Item = u16>,
    ) -> Result<(), RasterizationError> {
        font_map.with(font_id, |font| {
            self.fetch_batch_into_unchecked(device, queue, font, font_id, style, buf, gids)
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
        gids: impl Iterator<Item = u16>,
    ) -> Result<(), RasterizationError> {
        buf.reserve(gids.size_hint().0);

        let mut scope = self.rcx.open(RasterStyle {
            font: font.clone(),
            // use normalized values
            size: style.size.into(),
            weight: style.weight.into(),
            italic: style.italic,
            hint: style.hint,
            theme: style.theme,
        });

        for gid in gids {
            let data = self.cache.try_fetch(
                GlyphKey {
                    font: font_id,
                    glyph: gid,
                    style,
                },
                |key| {
                    let image = scope.rasterize(
                        key.glyph,
                        key.style.x_fract.into(),
                        key.style.y_fract.into(),
                    )?;

                    let view = if image.colored {
                        self.colored_atlas.store(device, queue, &image)
                    } else {
                        self.atlas.store(device, queue, &image)
                    };

                    Ok(GlyphData {
                        colored: image.colored,
                        view,
                    })
                },
            )?;

            let part = if data.colored {
                self.colored_atlas.read(&data.view)
            } else {
                self.atlas.read(&data.view)
            };

            buf.push(part);
        }

        Ok(())
    }

    pub fn update(&mut self) {
        for (_, data) in self.cache.sweep() {
            if data.colored {
                self.colored_atlas.remove(data.view);
            } else {
                self.atlas.remove(data.view);
            }
        }

        self.cache.tick();
    }
}
