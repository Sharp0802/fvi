use std::ops::Range;

use swash::text::cluster::Boundary;

use super::shape::{self, Cluster, Fragment, Kind, PositionedRun};
use super::{FontMap, LayoutError, Text};
use crate::Dp;

/// Additional line breaking within a paragraph.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Wrap {
    /// Only mandatory breaks produce new lines.
    None,
    /// Break at Unicode line opportunities; oversized words may overflow.
    Word,
    /// Also break oversized words at shaping cluster boundaries.
    #[default]
    WordOrCluster,
}

/// Horizontal alignment of each line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Alignment {
    /// Align at the left edge.
    #[default]
    Start,
    /// Center within the available width.
    Center,
    /// Align at the right edge.
    End,
}

/// Options for CPU text layout, independent of rasterization scale.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutDescriptor {
    /// Available width.
    pub width: Option<Dp>,
    /// Soft wrapping behavior.
    pub wrap: Wrap,
    /// Layout alignment.
    pub alignment: Alignment,
    /// Minimum line height.
    pub min_line_height: Option<Dp>,
    /// Accessibility font scale. See also [`Sp::to_dp`](`crate::Sp::to_dp`).
    pub font_scale: f32,
}

impl Default for LayoutDescriptor {
    fn default() -> Self {
        Self {
            width: None,
            wrap: Wrap::WordOrCluster,
            alignment: Alignment::Start,
            min_line_height: None,
            font_scale: 1.0,
        }
    }
}

/// The layout box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutBounds {
    /// Alignment width.
    pub width: Dp,
    /// Sum of the line heights.
    pub height: Dp,
}

/// Metrics for one line.
#[derive(Clone, Debug, PartialEq)]
pub struct LineMetrics {
    /// Original UTF-8 byte range.
    pub range: Range<usize>,
    /// Original byte range of the mandatory break.
    pub break_range: Option<Range<usize>>,
    /// Horizontal alignment offset.
    pub x: Dp,
    /// Top of the line box.
    pub y: Dp,
    /// Advance width excluding trailing breakable spaces.
    pub width: Dp,
    /// Full advance width.
    pub advance: Dp,
    /// Full line height.
    pub height: Dp,
    /// Baseline position relative to the layout origin.
    pub baseline: Dp,
    /// Maximum font ascent among the participating runs.
    pub ascent: Dp,
    /// Maximum font descent among the participating runs.
    pub descent: Dp,
    /// Maximum font leading among the participating runs.
    pub leading: Dp,
}

/// An owned layout.
#[derive(Clone, Debug)]
pub struct Layout {
    /// Logical bounds.
    pub bounds: LayoutBounds,
    /// Lines in source order.
    pub lines: Vec<LineMetrics>,
    pub(crate) runs: Vec<PositionedRun>,
}

pub(super) fn layout(
    text: &Text,
    fonts: &FontMap,
    desc: &LayoutDescriptor,
) -> Result<Layout, LayoutError> {
    validate(text, desc)?;
    let clusters = shape::prepare(text, fonts)?;
    let mut layout = Layout {
        bounds: LayoutBounds {
            width: Dp(0.0),
            height: Dp(0.0),
        },
        lines: Vec::new(),
        runs: Vec::new(),
    };
    let mut run_ranges = Vec::new();
    let mut start = 0;

    while start < clusters.len() {
        let end = clusters[start..]
            .iter()
            .position(|cluster| cluster.kind == Kind::Break)
            .map_or(clusters.len(), |offset| start + offset);
        let (mut next, mut fragment) = choose_line(text, fonts, &clusters, start..end, desc)?;
        let mut break_range = None;

        if next == end
            && let Some(hard_break) = clusters.get(end)
        {
            break_range = Some(hard_break.range.clone());

            let metrics = shape::fragment(text, fonts, &clusters[end..=end], desc.font_scale)?;
            fragment.ascent = fragment.ascent.max(metrics.ascent);
            fragment.descent = fragment.descent.max(metrics.descent);
            fragment.leading = fragment.leading.max(metrics.leading);
            next += 1;
        }

        let content_end = break_range
            .as_ref()
            .map_or_else(|| clusters[next - 1].range.end, |range| range.start);
        run_ranges.push(append_line(
            &mut layout,
            desc,
            fragment,
            clusters[start].range.start..content_end,
            break_range,
        ));
        start = next;
    }

    if let Some(last) = clusters.last()
        && last.kind == Kind::Break
    {
        let fragment = shape::fragment(text, fonts, std::slice::from_ref(last), desc.font_scale)?;
        run_ranges.push(append_line(
            &mut layout,
            desc,
            fragment,
            text.string.len()..text.string.len(),
            None,
        ));
    }

    let measured = layout
        .lines
        .iter()
        .map(|line| line.width.0)
        .fold(0.0_f32, f32::max);
    let width = desc.width.map_or(measured, |width| width.0);
    if !layout.lines.is_empty() {
        layout.bounds.width = Dp(width.max(measured));
    }

    for (line, runs) in layout.lines.iter_mut().zip(run_ranges) {
        let extra = (width - line.width.0).max(0.0);
        line.x = Dp(match desc.alignment {
            Alignment::Start => 0.0,
            Alignment::Center => extra * 0.5,
            Alignment::End => extra,
        });

        for run in &mut layout.runs[runs] {
            for glyph in &mut run.glyphs {
                glyph.x += line.x;
                glyph.y += line.baseline;
            }
        }
    }

    Ok(layout)
}

fn validate(text: &Text, desc: &LayoutDescriptor) -> Result<(), LayoutError> {
    let nonnegative = |value: f32| value.is_finite() && value >= 0.0;
    if desc.width.is_some_and(|width| !nonnegative(width.0))
        || desc
            .min_line_height
            .is_some_and(|height| !nonnegative(height.0))
        || !desc.font_scale.is_finite()
        || desc.font_scale <= 0.0
        || text.spans.iter().any(|span| {
            !nonnegative(span.style.size.0)
                || !nonnegative(span.style.size.to_dp(desc.font_scale).0)
        })
    {
        return Err(LayoutError::InvalidDimensions);
    }

    Ok(())
}

fn choose_line(
    text: &Text,
    fonts: &FontMap,
    clusters: &[Cluster],
    range: Range<usize>,
    desc: &LayoutDescriptor,
) -> Result<(usize, Fragment), LayoutError> {
    let shape = |end| shape::fragment(text, fonts, &clusters[range.start..end], desc.font_scale);
    let Some(width) = desc.width.filter(|_| desc.wrap != Wrap::None) else {
        return Ok((range.end, shape(range.end)?));
    };
    if range.is_empty() {
        return Ok((range.end, Fragment::default()));
    }

    let mut best = None;
    let breaks = (range.start + 1..range.end)
        .filter(|&end| clusters[end].boundary >= Boundary::Line)
        .chain(std::iter::once(range.end));

    for end in breaks {
        let fragment = shape(end)?;
        if fragment.width <= width.0 {
            best = Some((end, fragment));
            continue;
        }
        if let Some(best) = best {
            return Ok(best);
        }
        if desc.wrap == Wrap::Word {
            return Ok((end, fragment));
        }

        let mut fallback = None;
        for split in range.start + 1..=end {
            let mut candidate = shape(split)?;
            if candidate.width > width.0 {
                if let Some(fallback) = fallback {
                    return Ok(fallback);
                }

                let mut next = split;
                while next < end && clusters[next].space {
                    next += 1;
                }
                if next != split {
                    candidate = shape(next)?;
                }

                return Ok((next, candidate));
            }

            fallback = Some((split, candidate));
        }

        return Ok(fallback.unwrap());
    }

    Ok(best.unwrap())
}

fn append_line(
    layout: &mut Layout,
    desc: &LayoutDescriptor,
    fragment: Fragment,
    range: Range<usize>,
    break_range: Option<Range<usize>>,
) -> Range<usize> {
    let natural = fragment.ascent + fragment.descent + fragment.leading;
    let height = desc
        .min_line_height
        .map_or(natural, |height| height.0.max(natural));
    let y = layout.bounds.height;
    let baseline = (fragment.leading + height - natural).mul_add(0.5, y.0 + fragment.ascent);

    layout.lines.push(LineMetrics {
        range,
        break_range,
        x: Dp(0.0),
        y,
        width: Dp(fragment.width),
        advance: Dp(fragment.advance),
        height: Dp(height),
        baseline: Dp(baseline),
        ascent: Dp(fragment.ascent),
        descent: Dp(fragment.descent),
        leading: Dp(fragment.leading),
    });
    layout.bounds.height += Dp(height);

    let start = layout.runs.len();
    layout.runs.extend(fragment.runs);
    start..layout.runs.len()
}
