use crate::piece::Piece;

#[derive(Debug, Clone)]
pub struct State {
    pub oldest: u32,
    pub latest: u32,
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
    pub const fn new(version: u32) -> Self {
        assert!(version != u32::MAX);
        Self {
            oldest: version,
            latest: version,
        }
    }

    /// Sets latest version as given,
    /// returning whether the eager pruning should be called.
    #[must_use]
    pub fn invalidate(&mut self, version: u32) -> bool {
        self.oldest = self.oldest.min(version);
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
