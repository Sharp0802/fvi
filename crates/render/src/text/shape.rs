use harfrust::{BufferFlags, Direction, ShaperData, ShaperInstance, UnicodeBuffer};
use std::ops::Range;
use swash::text::cluster::{Boundary, CharCluster, CharInfo, Parser, Token};
use swash::text::{ClusterBreak, Codepoint, LineBreak, Script, analyze};

use super::{FontId, FontMap, FontRef, LayoutError, Style, Text};
use crate::Dp;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Text,
    Tab,
    Break,
}

pub(super) struct Cluster {
    pub range: Range<usize>,
    pub style: usize,
    pub script: Script,
    pub boundary: Boundary,
    pub kind: Kind,
    pub space: bool,
    pub font: Option<FontId>,
    replacement: bool,
    chars: CharCluster,
}

#[derive(Clone, Debug)]
#[expect(
    clippy::redundant_pub_crate,
    reason = "positioned glyphs remain crate-private"
)]
#[expect(dead_code, reason = "glyph IDs are retained for future rendering")]
pub(crate) struct PositionedGlyph {
    pub id: u16,
    pub range: Range<usize>,
    pub x: Dp,
    pub y: Dp,
    pub advance: Dp,
}

#[derive(Clone, Debug)]
#[expect(
    clippy::redundant_pub_crate,
    reason = "positioned runs remain crate-private"
)]
#[expect(
    dead_code,
    reason = "font handles and styles are retained for future rendering"
)]
pub(crate) struct PositionedRun {
    pub font: FontId,
    pub style: Style,
    pub range: Range<usize>,
    pub glyphs: Vec<PositionedGlyph>,
}

#[derive(Debug, Default)]
pub(super) struct Fragment {
    pub runs: Vec<PositionedRun>,
    pub advance: f32,
    pub width: f32,
    pub ascent: f32,
    pub descent: f32,
    pub leading: f32,
}

impl Fragment {
    const fn include_metrics(&mut self, metrics: swash::Metrics) {
        self.ascent = self.ascent.max(metrics.ascent);
        self.descent = self.descent.max(metrics.descent);
        self.leading = self.leading.max(metrics.leading);
    }
}

const fn kind(ch: char) -> Kind {
    match ch {
        '\r' | '\n' | '\u{b}' | '\u{c}' | '\u{85}' | '\u{2028}' | '\u{2029}' => Kind::Break,
        '\t' => Kind::Tab,
        _ => Kind::Text,
    }
}

const fn neutral(script: Script) -> bool {
    matches!(script, Script::Common | Script::Inherited | Script::Unknown)
}

fn analyze_text(text: &Text) -> Result<Vec<Cluster>, LayoutError> {
    u32::try_from(text.string.len()).map_err(|_| LayoutError::TextTooLong)?;

    let mut analysis = analyze(text.string.chars());
    let mut tokens = Vec::new();
    let mut scripts = Vec::new();
    for ((offset, ch), (properties, boundary)) in text.string.char_indices().zip(&mut analysis) {
        tokens.push(Token {
            ch,
            offset: u32::try_from(offset).map_err(|_| LayoutError::TextTooLong)?,
            len: u8::try_from(ch.len_utf8()).unwrap(),
            info: CharInfo::new(properties, boundary),
            data: 0,
        });
        scripts.push(match properties.cluster_break() {
            ClusterBreak::EX | ClusterBreak::SM | ClusterBreak::ZWJ => Script::Inherited,
            _ => properties.script(),
        });
    }

    if analysis.needs_bidi_resolution() {
        return Err(LayoutError::UnsupportedBidi);
    }

    resolve_scripts(&tokens, &mut scripts);

    let mut clusters: Vec<Cluster> = Vec::new();
    let mut start = 0;
    while start < tokens.len() {
        let token = tokens[start];
        let token_kind = kind(token.ch);
        let mut end = start + 1;

        if token_kind == Kind::Break && token.ch == '\r' {
            if tokens.get(end).is_some_and(|next| next.ch == '\n') {
                end += 1;
            }
        } else if token_kind == Kind::Text {
            while end < tokens.len()
                && kind(tokens[end].ch) == Kind::Text
                && scripts[end] == scripts[start]
            {
                end += 1;
            }
        }

        let mut parser = Parser::new(scripts[start], tokens[start..end].iter().copied());
        let mut chars = CharCluster::new();
        while parser.next(&mut chars) {
            let range = chars.range().to_range();
            let source = &text.string[range.clone()];

            // swash's fixed-capacity parser overlaps chunks of long clusters.
            // its simple parser also emits regional indicators individually.
            if let Some(last) = clusters.last_mut() {
                let previous = &text.string[last.range.clone()];
                let flag = previous.chars().count() == 1
                    && previous.chars().all(regional_indicator)
                    && source.chars().all(regional_indicator);

                if range.start < last.range.end || flag {
                    last.range.end = range.end;
                    continue;
                }
            }

            clusters.push(Cluster {
                range,
                style: 0,
                script: scripts[start],
                boundary: chars.info().boundary(),
                kind: token_kind,
                space: source.chars().all(|ch| {
                    ch.is_whitespace() && !matches!(ch.line_break(), LineBreak::GL | LineBreak::WJ)
                }),
                font: None,
                replacement: false,
                chars,
            });
        }
        start = end;
    }

    let mut style = 0;
    for cluster in &mut clusters {
        while text.spans[style].range.end <= cluster.range.start {
            style += 1;
        }

        cluster.style = style;
        if cluster.kind != Kind::Break && text.spans[style].range.end < cluster.range.end {
            return Err(LayoutError::StyleInsideCluster(text.spans[style].range.end));
        }
    }

    Ok(clusters)
}

fn resolve_scripts(tokens: &[Token], scripts: &mut [Script]) {
    // prefer the preceding script, then the following script within the paragraph.
    let mut previous = Script::Common;
    for (token, script) in tokens.iter().zip(scripts.iter_mut()) {
        if kind(token.ch) == Kind::Break {
            previous = Script::Common;
        } else if neutral(*script) {
            *script = previous;
        } else {
            previous = *script;
        }
    }

    let mut following = Script::Common;
    for (token, script) in tokens.iter().zip(scripts.iter_mut()).rev() {
        if kind(token.ch) == Kind::Break {
            following = Script::Common;
        } else if neutral(*script) {
            *script = following;
        } else {
            following = *script;
        }
    }
}

const fn regional_indicator(ch: char) -> bool {
    matches!(ch, '\u{1f1e6}'..='\u{1f1ff}')
}

fn char_cluster(ch: char) -> CharCluster {
    let token = Token {
        ch,
        len: u8::try_from(ch.len_utf8()).unwrap(),
        info: ch.into(),
        ..Token::default()
    };
    let mut cluster = CharCluster::new();
    Parser::new(Script::Common, std::iter::once(token)).next(&mut cluster);
    cluster
}

fn resolve_font(fonts: &FontMap, style: &Style, chars: &mut CharCluster) -> Option<(FontId, bool)> {
    if let Some(font) = style.fonts.resolve(fonts, chars) {
        return Some((font, false));
    }

    let font = style
        .fonts
        .resolve(fonts, &mut char_cluster('\u{fffd}'))
        .or_else(|| style.fonts.resolve(fonts, &mut CharCluster::new()))?;
    Some((font, true))
}

pub(super) fn prepare(text: &Text, fonts: &FontMap) -> Result<Vec<Cluster>, LayoutError> {
    let mut clusters = analyze_text(text)?;
    for cluster in &mut clusters {
        let mut chars = if cluster.kind == Kind::Text {
            cluster.chars
        } else {
            char_cluster(' ')
        };
        let style = &text.spans[cluster.style].style;
        let (font, replacement) = resolve_font(fonts, style, &mut chars)
            .ok_or_else(|| LayoutError::MissingFont(cluster.range.clone()))?;
        cluster.font = Some(font);
        cluster.replacement = replacement;
    }

    Ok(clusters)
}

fn metrics(font: &FontRef, style: &Style, scale: f32) -> swash::Metrics {
    let font: &swash::FontRef = font.as_ref();
    let coords: Vec<_> = font
        .variations()
        .normalized_coords([
            ("wght", f32::from(style.weight)),
            ("ital", f32::from(style.italic)),
        ])
        .collect();
    font.metrics(&coords).scale(style.size.to_dp(scale).0)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "font design units are scaled to f32 layout coordinates"
)]
fn shape_run(
    font: &FontRef,
    style: &Style,
    scale: f32,
    source: &str,
    range: Range<usize>,
    context: Range<usize>,
    script: Script,
) -> Vec<PositionedGlyph> {
    let face: &harfrust::FontRef = font.as_ref();
    let data = ShaperData::new(face);
    let instance = ShaperInstance::from_variations(
        face,
        [
            (harfrust::Tag::new(b"wght"), f32::from(style.weight)),
            (harfrust::Tag::new(b"ital"), f32::from(style.italic)),
        ],
    );
    let shaper = data.shaper(face).instance(Some(&instance)).build();
    let factor = style.size.to_dp(scale).0 / shaper.units_per_em().max(1) as f32;

    let mut buffer = UnicodeBuffer::new();
    for (offset, ch) in source[range.clone()].char_indices() {
        buffer.add(ch, u32::try_from(range.start + offset).unwrap());
    }

    buffer.set_pre_context(&source[context.start..range.start]);
    buffer.set_post_context(&source[range.end..context.end]);
    buffer.set_direction(Direction::LeftToRight);
    buffer.set_script(harfrust_script(script));

    let mut flags = BufferFlags::empty();
    if range.start == context.start {
        flags |= BufferFlags::BEGINNING_OF_TEXT;
    }
    if range.end == context.end {
        flags |= BufferFlags::END_OF_TEXT;
    }
    buffer.set_flags(flags);

    let buffer = shaper.shape(buffer, harfrust::ShapeOptions::default());
    let infos = buffer.glyph_infos();
    let mut boundaries: Vec<_> = infos.iter().map(|info| info.cluster as usize).collect();
    boundaries.push(range.end);
    boundaries.sort_unstable();
    boundaries.dedup();

    let mut x = 0.0;
    let mut y = 0.0;
    infos
        .iter()
        .zip(buffer.glyph_positions())
        .map(|(info, position)| {
            let start = info.cluster as usize;
            let end = boundaries[boundaries.partition_point(|&offset| offset <= start)];
            let glyph = PositionedGlyph {
                id: u16::try_from(info.glyph_id).unwrap(),
                range: start..end,
                x: Dp((position.x_offset as f32).mul_add(factor, x)),
                y: Dp((position.y_offset as f32).mul_add(-factor, y)),
                advance: Dp(position.x_advance as f32 * factor),
            };

            x += glyph.advance.0;
            y = (position.y_advance as f32).mul_add(-factor, y);
            glyph
        })
        .collect()
}

fn harfrust_script(script: Script) -> harfrust::Script {
    // these tags differ from their ISO 15924 names.
    let tag = match script {
        Script::Bengali => *b"Beng",
        Script::Devanagari => *b"Deva",
        Script::Gujarati => *b"Gujr",
        Script::Gurmukhi => *b"Guru",
        Script::Kannada => *b"Knda",
        Script::Malayalam => *b"Mlym",
        Script::Myanmar => *b"Mymr",
        Script::Oriya => *b"Orya",
        Script::Tamil => *b"Taml",
        _ if neutral(script) => return harfrust::script::COMMON,
        _ => script.to_opentype().to_be_bytes(),
    };
    harfrust::Script::from_iso15924_tag(harfrust::Tag::new(&tag))
        .unwrap_or(harfrust::script::UNKNOWN)
}

fn space_width(fonts: &FontMap, style: &Style, scale: f32, script: Script) -> Option<f32> {
    let (id, replacement) = resolve_font(fonts, style, &mut char_cluster(' '))?;
    let source = if replacement { "\u{fffd}" } else { " " };
    let range = 0..source.len();
    let glyphs = fonts.with(id, |font| {
        shape_run(font, style, scale, source, range.clone(), range, script)
    });
    Some(glyphs.iter().map(|glyph| glyph.advance.0).sum())
}

pub(super) fn fragment(
    text: &Text,
    fonts: &FontMap,
    clusters: &[Cluster],
    scale: f32,
) -> Result<Fragment, LayoutError> {
    let mut fragment = Fragment::default();
    let Some(first) = clusters.first() else {
        return Ok(fragment);
    };

    let end = clusters
        .iter()
        .rfind(|cluster| cluster.kind != Kind::Break)
        .map_or(first.range.start, |cluster| cluster.range.end);
    let context = first.range.start..end;
    let visible_end = clusters
        .iter()
        .rfind(|cluster| !cluster.space && cluster.kind != Kind::Break)
        .map_or(context.start, |cluster| cluster.range.end);
    let mut tab_width = None;
    let mut start = 0;

    while start < clusters.len() {
        let cluster = &clusters[start];
        let style = &text.spans[cluster.style].style;
        let font = cluster.font.unwrap();
        fragment.include_metrics(fonts.with(font, |font| metrics(font, style, scale)));

        if cluster.kind == Kind::Tab {
            let stop = if let Some(width) = tab_width {
                width
            } else {
                let initial_style = &text.spans[first.style].style;
                let width = space_width(fonts, initial_style, scale, first.script)
                    .ok_or_else(|| LayoutError::MissingFont(first.range.clone()))?
                    * 4.0;
                tab_width = Some(width);
                width
            };

            if stop > 0.0 {
                fragment.advance = (fragment.advance / stop).floor().mul_add(stop, stop);
            }
            if cluster.range.end <= visible_end {
                fragment.width = fragment.advance;
            }
            start += 1;
            continue;
        }

        if cluster.kind == Kind::Break {
            start += 1;
            continue;
        }

        let mut end = start + 1;
        while !cluster.replacement
            && end < clusters.len()
            && !clusters[end].replacement
            && clusters[end].kind == Kind::Text
            && clusters[end].style == cluster.style
            && clusters[end].font == cluster.font
            && clusters[end].script == cluster.script
        {
            end += 1;
        }

        let range = cluster.range.start..clusters[end - 1].range.end;
        let (source, source_range, run_context, script) = if cluster.replacement {
            ("\u{fffd}", 0..3, 0..3, Script::Common)
        } else {
            (
                text.string.as_str(),
                range.clone(),
                context.clone(),
                cluster.script,
            )
        };
        let mut glyphs = fonts.with(font, |font| {
            shape_run(
                font,
                style,
                scale,
                source,
                source_range,
                run_context,
                script,
            )
        });

        let origin = fragment.advance;
        for glyph in &mut glyphs {
            if cluster.replacement {
                glyph.range = range.clone();
            }
            glyph.x.0 += origin;
            fragment.advance += glyph.advance.0;
            if glyph.range.start < visible_end {
                fragment.width = fragment.advance;
            }
        }

        fragment.runs.push(PositionedRun {
            font,
            style: style.clone(),
            range,
            glyphs,
        });
        start = end;
    }

    Ok(fragment)
}
