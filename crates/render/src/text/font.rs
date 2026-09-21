use std::fmt::Debug;
use std::sync::Arc;
use swash::text::cluster::{CharCluster, Status};

#[expect(
    clippy::redundant_pub_crate,
    reason = "it's crate-scoped although parent is pub"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FontId(fontdb::ID);

#[expect(
    clippy::redundant_pub_crate,
    reason = "it's crate-scoped although parent is pub"
)]
#[derive(Clone)]
pub(crate) struct FontRef<'a> {
    harfrust: harfrust::FontRef<'a>,
    swash: swash::FontRef<'a>,
}

impl<'a> FontRef<'a> {
    pub fn from_index(data: &'a [u8], index: u32) -> Option<Self> {
        Some(Self {
            harfrust: harfrust::FontRef::from_index(data, index).ok()?,
            swash: swash::FontRef::from_index(data, index as usize)?,
        })
    }
}

impl Debug for FontRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontRef")
            .field("data", &self.swash.data)
            .field("offset", &self.swash.offset)
            .finish_non_exhaustive()
    }
}

impl<'a> AsRef<harfrust::FontRef<'a>> for FontRef<'a> {
    fn as_ref(&self) -> &harfrust::FontRef<'a> {
        &self.harfrust
    }
}

impl<'a> AsRef<swash::FontRef<'a>> for FontRef<'a> {
    fn as_ref(&self) -> &swash::FontRef<'a> {
        &self.swash
    }
}

impl<'a> From<FontRef<'a>> for harfrust::FontRef<'a> {
    fn from(value: FontRef<'a>) -> Self {
        value.harfrust
    }
}

impl<'a> From<FontRef<'a>> for swash::FontRef<'a> {
    fn from(value: FontRef<'a>) -> Self {
        value.swash
    }
}

/// A set of fonts.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Fonts {
    fonts: Arc<[fontdb::ID]>,
}

impl Fonts {
    pub(crate) fn resolve(&self, map: &FontMap, cluster: &mut CharCluster) -> Option<FontId> {
        let mut best = None;
        for &font in &*self.fonts {
            match map
                .db
                .with_face_data(font, |data, index| {
                    let Some(font) = swash::FontRef::from_index(data, index as usize) else {
                        return Status::Discard;
                    };

                    let charmap = font.charmap();
                    cluster.map(|ch| charmap.map(ch))
                })
                .unwrap_or(Status::Discard)
            {
                Status::Complete => return Some(FontId(font)),
                Status::Keep => best = Some(FontId(font)),
                Status::Discard => {}
            }
        }

        best
    }
}

/// A style of the font.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontStyle {
    /// Font weight.
    pub weight: u16,
    /// Whether to set italic.
    pub italic: bool,
}

/// A descriptor of [`Fonts`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontsDescriptor<'a> {
    /// Names of fonts.
    pub names: &'a [&'a str],
    /// Font style.
    pub style: FontStyle,
}

impl Fonts {
    fn new(map: &FontMap, desc: &FontsDescriptor) -> Self {
        let mut fonts = Vec::with_capacity(desc.names.len());
        for name in desc.names {
            #[rustfmt::skip]
            let family = match *name {
                "serif"      => fontdb::Family::Serif,
                "sans-serif" => fontdb::Family::SansSerif,
                "monospace"  => fontdb::Family::Monospace,
                "cursive"    => fontdb::Family::Cursive,
                "fantasy"    => fontdb::Family::Fantasy,
                name         => fontdb::Family::Name(name),
            };

            let Some(font_id) = map.db.query(&fontdb::Query {
                families: &[family],
                weight: fontdb::Weight(desc.style.weight),
                stretch: fontdb::Stretch::Normal,
                style: if desc.style.italic {
                    fontdb::Style::Italic
                } else {
                    fontdb::Style::Normal
                },
            }) else {
                continue;
            };

            fonts.push(font_id);
        }

        Self {
            fonts: fonts.into_boxed_slice().into(),
        }
    }
}

/// A map of available fonts.
#[derive(Debug)]
pub struct FontMap {
    db: fontdb::Database,
}

impl FontMap {
    pub(crate) fn new() -> Self {
        Self {
            db: fontdb::Database::new(),
        }
    }

    /// Resolves a [`FontsDescriptor`] to [`Fonts`].
    #[must_use]
    pub fn resolve(&self, desc: &FontsDescriptor) -> Fonts {
        Fonts::new(self, desc)
    }

    pub(crate) fn with<T, F>(&self, id: FontId, f: F) -> T
    where
        F: FnOnce(&FontRef) -> T,
    {
        self.db
            .with_face_data(id.0, |data, index| {
                let font_ref = FontRef::from_index(data, index).unwrap();
                f(&font_ref)
            })
            .expect("id is not from self")
    }
}

impl Default for FontMap {
    fn default() -> Self {
        Self::new()
    }
}
