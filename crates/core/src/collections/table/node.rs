use crate::piece::Piece;

#[derive(Debug, Clone, Copy)]
pub struct Node {
    pub len: Option<u64>,
    pub prv: usize,
    pub lhs: usize,
    pub rhs: usize,
    pub val: Piece,
}

impl From<Piece> for Node {
    fn from(value: Piece) -> Self {
        Self {
            len: Some(value.len()),
            prv: usize::MAX,
            lhs: usize::MAX,
            rhs: usize::MAX,
            val: value,
        }
    }
}
