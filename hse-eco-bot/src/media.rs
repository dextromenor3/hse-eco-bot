#[derive(Debug, Clone, Eq, PartialEq)]
pub struct File {
    pub id: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Image {
    pub file: File,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Video {
    pub file: File,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Location {
    pub longitude: f64,
    pub latitude: f64,
    pub accuracy: Option<f64>,
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}", self.longitude, self.latitude)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Document {
    pub file: File,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LocationOrAddress {
    Location(Location),
    Address(String),
}

impl std::fmt::Display for LocationOrAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Location(loc) => write!(f, "{}", loc),
            Self::Address(address) => write!(f, "{}", address),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Attachment {
    Image(Image),
    Video(Video),
    Document(Document),
}
