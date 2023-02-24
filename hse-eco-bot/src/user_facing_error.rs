use crate::message::FormattedText;
use std::error::Error;

pub trait UserFacingError: Error {
    fn user_message(&self) -> FormattedText;
}
