/// An identifier of an element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Id {
    /// A source location. See [`Location`].
    Location(Location),
}

impl Default for Id {
    #[track_caller]
    fn default() -> Self {
        Self::Location(Location::default())
    }
}

/// A source location.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Location {
    file: &'static str,
    line: u32,
    column: u32,
}

impl Default for Location {
    #[track_caller]
    fn default() -> Self {
        let loc = std::panic::Location::caller();
        Self {
            file: loc.file(),
            line: loc.line(),
            column: loc.column(),
        }
    }
}
