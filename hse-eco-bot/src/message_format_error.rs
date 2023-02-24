use crate::strings::STRINGS;
use crate::user_facing_error::UserFacingError;
use std::error::Error;
use std::fmt::Display;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MessageFormatError {
    NoText,
    HasAttachments,
    InvalidName,
}

impl Display for MessageFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoText => write!(f, "The message has no text"),
            Self::HasAttachments => write!(f, "The message has attachments that were not expected"),
            Self::InvalidName => {
                write!(f, "The message text is invalid as a note or directory name")
            }
        }
    }
}

impl Error for MessageFormatError {}

impl UserFacingError for MessageFormatError {
    fn user_message(&self) -> crate::message::FormattedText {
        let s = &STRINGS.errors.message_format;
        match self {
            Self::NoText => s.no_text(),
            Self::HasAttachments => s.has_attachments(),
            Self::InvalidName => s.invalid_name(),
        }
    }
}
