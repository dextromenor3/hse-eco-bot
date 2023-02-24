use crate::message::FormattedText;
use crate::strings::STRINGS;
use crate::user_facing_error::UserFacingError;
use std::error::Error;
use std::fmt::Display;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum InvalidAction {
    CannotGoUp,
    InvalidState,
    UnexpectedMessage,
    UnexpectedMessageKind,
}

impl Display for InvalidAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CannotGoUp => write!(f, "Cannot go up from the root KB directory"),
            Self::InvalidState => write!(f, "Invalid state for selected action"),
            Self::UnexpectedMessage => write!(f, "A message was received when it was not expected"),
            Self::UnexpectedMessageKind => write!(f, "An unexpected type of message was received"),
        }
    }
}

impl Error for InvalidAction {}

impl UserFacingError for InvalidAction {
    fn user_message(&self) -> FormattedText {
        match self {
            Self::CannotGoUp => STRINGS.errors.action.cannot_go_up(),
            Self::InvalidState => STRINGS.errors.action.invalid_state(),
            Self::UnexpectedMessage => STRINGS.errors.action.unexpected_message(),
            Self::UnexpectedMessageKind => STRINGS.errors.action.unexpected_message_kind(),
        }
    }
}
