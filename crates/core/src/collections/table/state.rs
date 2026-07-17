use crate::collections::table::{Context, Version};
use crate::piece::Piece;

#[derive(Debug, Clone)]
pub struct State {
    pub oldest: Version,
    pub latest: Version,
}

#[derive(Debug, Clone, Copy)]
pub enum Verdict {
    None,
    Update,
    Kill,
    Revive,
}

impl State {
    #[must_use]
    pub const fn new(cx: &Context) -> Self {
        Self {
            oldest: cx.version,
            latest: cx.version,
        }
    }

    /// Updates this [`State`] with the given context,
    /// returning whether the eager pruning should be called.
    #[must_use]
    pub fn update(&mut self, cx: &Context) -> bool {
        self.oldest = self.oldest.max(cx.possible_oldest());
        let top = core::mem::replace(&mut self.latest, cx.version);
        top > cx.version
    }

    #[must_use]
    pub fn verdict(&self, piece: &Piece) -> Verdict {
        if piece.del_at.is_some_and(|t| t < self.oldest) || self.latest < piece.add_at {
            Verdict::Kill
        } else if piece.del_at.is_some_and(|t| t > self.latest) {
            Verdict::Revive
        } else if piece.add_at < self.oldest {
            Verdict::Update
        } else {
            Verdict::None
        }
    }
}
