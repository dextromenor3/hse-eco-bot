use crate::dispatch::InvalidChatError;
use crate::invalid_action::InvalidAction;
use crate::kb::ProviderError;
use crate::message::FormattedText;
use crate::message_format_error::MessageFormatError;
use crate::user_facing_error::UserFacingError;
use std::error::Error;
use crate::ui::form::FormInputError;
use std::fmt::Display;

#[derive(Debug, PartialEq)]
pub enum UserError {
    Provider(ProviderError),
    InvalidChat(InvalidChatError),
    InvalidAction(InvalidAction),
    MessageFormat(MessageFormatError),
    FormInput(FormInputError),
}

impl Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Provider(e) => Display::fmt(&e, f),
            Self::InvalidChat(e) => Display::fmt(&e, f),
            Self::InvalidAction(e) => Display::fmt(&e, f),
            Self::MessageFormat(e) => Display::fmt(&e, f),
            Self::FormInput(e) => Display::fmt(&e, f),
        }
    }
}

impl Error for UserError {}

impl From<ProviderError> for UserError {
    fn from(e: ProviderError) -> Self {
        Self::Provider(e)
    }
}

impl From<InvalidChatError> for UserError {
    fn from(e: InvalidChatError) -> Self {
        Self::InvalidChat(e)
    }
}

impl From<InvalidAction> for UserError {
    fn from(e: InvalidAction) -> Self {
        Self::InvalidAction(e)
    }
}

impl From<MessageFormatError> for UserError {
    fn from(e: MessageFormatError) -> Self {
        Self::MessageFormat(e)
    }
}

impl From<FormInputError> for UserError {
    fn from(e: FormInputError) -> Self {
        Self::FormInput(e)
    }
}

impl UserFacingError for UserError {
    fn user_message(&self) -> FormattedText {
        match self {
            Self::Provider(e) => e.user_message(),
            Self::InvalidChat(e) => e.user_message(),
            Self::InvalidAction(e) => e.user_message(),
            Self::MessageFormat(e) => e.user_message(),
            Self::FormInput(e) => e.user_message(),
        }
    }
}
