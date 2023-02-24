use crate::dispatch::InvalidChatError;
use crate::invalid_action::InvalidAction;
use crate::ui::form::FormInputError;
use crate::kb::ProviderError;
use crate::message_format_error::MessageFormatError;
use crate::user_error::UserError;
use std::error::Error;
use std::fmt::Display;
use teloxide::adaptors::{AutoSend, Throttle};
use teloxide::{Bot, RequestError};

/// Type alias for the actual bot type used.
pub type BotType = AutoSend<Bot>;

/// The non-user error type of a dialog state handler.
#[derive(Debug)]
pub enum InternalError {
    Teloxide(RequestError),
}

impl Display for InternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Teloxide(e) => write!(f, "Telegram Bot API Error: {}", e),
        }
    }
}

impl Error for InternalError {}

impl From<RequestError> for InternalError {
    fn from(e: RequestError) -> Self {
        Self::Teloxide(e)
    }
}

/// The error type of a dialog state handler.
#[derive(Debug)]
pub enum HandlerError {
    Internal(InternalError),
    User(UserError),
}

impl Display for HandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Internal(e) => write!(f, "{}", e),
            Self::User(e) => write!(f, "{}", e),
        }
    }
}

impl Error for HandlerError {}

impl From<InternalError> for HandlerError {
    fn from(e: InternalError) -> Self {
        Self::Internal(e)
    }
}

impl From<UserError> for HandlerError {
    fn from(e: UserError) -> Self {
        Self::User(e)
    }
}

impl From<RequestError> for HandlerError {
    fn from(e: RequestError) -> Self {
        InternalError::from(e).into()
    }
}

impl From<ProviderError> for HandlerError {
    fn from(e: ProviderError) -> Self {
        UserError::from(e).into()
    }
}

impl From<InvalidChatError> for HandlerError {
    fn from(e: InvalidChatError) -> Self {
        UserError::from(e).into()
    }
}

impl From<InvalidAction> for HandlerError {
    fn from(e: InvalidAction) -> Self {
        UserError::from(e).into()
    }
}

impl From<MessageFormatError> for HandlerError {
    fn from(e: MessageFormatError) -> Self {
        UserError::from(e).into()
    }
}

impl From<FormInputError> for HandlerError {
    fn from(e: FormInputError) -> Self {
        UserError::from(e).into()
    }
}

/// The result type of a dialog state handler.
pub type HandlerResult<T> = Result<T, HandlerError>;
