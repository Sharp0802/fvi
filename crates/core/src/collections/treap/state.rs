use crate::piece::Piece;

#[derive(Debug, Clone)]
pub struct State {
    max_diff: u32,
    oldest: u32,
    latest: u32,
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
    pub const fn new(max_diff: u32, version: u32) -> Self {
        assert!(version != u32::MAX, "invalid version constant");
        Self {
            max_diff,
            oldest: version,
            latest: version,
        }
    }

    #[must_use]
    pub const fn max_diff(&self) -> u32 {
        self.max_diff
    }

    #[must_use]
    pub const fn oldest(&self) -> u32 {
        self.oldest
    }

    #[must_use]
    pub const fn latest(&self) -> u32 {
        self.latest
    }

    /// Sets latest version as given,
    /// returning whether the eager pruning should be called.
    #[must_use]
    pub fn invalidate(&mut self, version: u32) -> bool {
        self.oldest = self.oldest.max(version.saturating_sub(self.max_diff));
        let top = core::mem::replace(&mut self.latest, version);
        top > version
    }

    #[must_use]
    pub const fn verdict(&self, piece: &Piece) -> Verdict {
        if piece.del_at < self.oldest || self.latest < piece.add_at {
            Verdict::Kill
        } else if piece.del_at > self.latest {
            Verdict::Revive
        } else if piece.add_at < self.oldest {
            Verdict::Update
        } else {
            Verdict::None
        }
    }
}
