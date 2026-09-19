use std::rc::Rc;

/// A reference of font.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontRef(Rc<Font>);

/// A font.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Font {}
