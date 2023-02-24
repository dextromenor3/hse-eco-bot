use crate::db::{FullDirectoryId, FullNoteId};
use crate::feedback::FeedbackTopic;
use lazy_static::lazy_static;
use regex::Regex;
use std::convert::TryFrom;
use std::error::Error;
use std::fmt::Display;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Query {
    /// Exit from anywhere and open the main menu.
    OpenMainMenu,
    /// Exit from anywhere and open the knowledge base root.
    OpenKb,
    /// Exit from anywhere and open the newsletter archive page in KB.
    OpenNewsletterArchive,
    /// Exit from anywhere and open the calendar tool.
    OpenCalendar,
    /// Exit from anywhere and open the feedback page.
    OpenFeedback,
    /// Exit from anywhere and open the feedback page with a topic
    /// pre-selected.
    OpenFeedbackTopic {
        topic: FeedbackTopic,
    },
    /// Exit from anywhere and open the newsletter settings page.
    OpenNlSettings,
    /// Go up in the knowledge base.
    KbGoUp,
    /// Navigate to a specific directory in the knowledge base.
    KbNavToDir {
        id: FullDirectoryId,
    },
    /// Navigate to a specific note in the knowledge base.
    KbNavToNote {
        id: FullNoteId,
    },
    /// Universal "go back" request.
    GoBack,
    /// Edit a note in the knowledge base.
    KbEditNote {
        id: FullNoteId,
    },
    /// Rename a note in the knowledge base.
    KbRenameNote {
        id: FullNoteId,
    },
    /// Move a note in the knowledge base.
    KbMoveNote {
        id: FullNoteId,
    },
    /// Delete a note in the knowledge base.
    KbDeleteNote {
        id: FullNoteId,
    },
    /// Pin a note in the knowledge base.
    KbPinNote {
        id: FullNoteId,
    },
    /// Unpin a note in the knowledge base.
    KbUnpinNote {
        id: FullNoteId,
    },
    /// Confirm note deletion.
    KbConfirmNoteDeletion {
        id: FullNoteId,
    },
    /// Cancel note deletion.
    KbCancelNoteDeletion {
        id: FullNoteId,
    },
    /// Edit a directory.
    KbEditDir {
        id: FullDirectoryId,
    },
    /// Create a note.
    KbCreateNote {
        destination: FullDirectoryId,
    },
    /// Move a note to the specified directory.
    KbMoveNoteHere {
        note: FullNoteId,
        destination: FullDirectoryId,
    },
    /// Move a directory to the specified directory.
    KbMoveDirectoryHere {
        directory: FullDirectoryId,
        destination: FullDirectoryId,
    },
    /// Move a directory in the knowledge base.
    KbMoveDirectory {
        id: FullDirectoryId,
    },
    /// Create a directory.
    KbCreateDirectory {
        destination: FullDirectoryId,
    },
    /// Rename a directory.
    KbRenameDirectory {
        id: FullDirectoryId,
    },
    /// Delete a directory.
    KbDeleteDirectory {
        id: FullDirectoryId,
    },
    /// Pin a directory.
    KbPinDirectory {
        id: FullDirectoryId,
    },
    /// Unpin a directory.
    KbUnpinDirectory {
        id: FullDirectoryId,
    },
    /// Confirm deletion of a directory.
    KbConfirmDirectoryDeletion {
        id: FullDirectoryId,
    },
    KbCancelDirectoryDeletion {
        id: FullDirectoryId,
    },
    FormOption {
        index: usize,
    },
    Subscribe {
        newsletter: String,
    },
    Unsubscribe {
        newsletter: String,
    },
    ManageSubscriptions,
}

impl Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use strings::cmd;

        match self {
            Self::OpenMainMenu => write!(f, "{}", cmd::OPEN_MAIN_MENU),
            Self::OpenKb => write!(f, "{}", cmd::OPEN_KB),
            Self::OpenNewsletterArchive => write!(f, "{}", cmd::OPEN_NL_ARCHIVE),
            Self::OpenCalendar => write!(f, "{}", cmd::OPEN_CALENDAR),
            Self::OpenFeedback => write!(f, "{}", cmd::OPEN_FEEDBACK),
            Self::OpenFeedbackTopic { topic } => {
                write!(f, "{}@{}", cmd::OPEN_FEEDBACK_TOPIC, topic)
            }
            Self::KbGoUp => write!(f, "{}", cmd::KB_GO_UP),
            Self::KbNavToDir { id } => {
                write!(f, "{}@{}", cmd::KB_NAV_TO_DIR, id)
            }
            Self::KbNavToNote { id } => {
                write!(f, "{}@{}", cmd::KB_NAV_TO_NOTE, id)
            }
            Self::OpenNlSettings => write!(f, "{}", cmd::OPEN_NL_SETTINGS),
            Self::GoBack => write!(f, "{}", cmd::GO_BACK),
            Self::KbEditNote { id } => write!(f, "{}@{}", cmd::KB_EDIT_NOTE, id),
            Self::KbRenameNote { id } => write!(f, "{}@{}", cmd::KB_RENAME_NOTE, id),
            Self::KbMoveNote { id } => write!(f, "{}@{}", cmd::KB_MOVE_NOTE, id),
            Self::KbDeleteNote { id } => write!(f, "{}@{}", cmd::KB_DELETE_NOTE, id),
            Self::KbPinNote { id } => write!(f, "{}@{}", cmd::KB_PIN_NOTE, id),
            Self::KbUnpinNote { id } => write!(f, "{}@{}", cmd::KB_UNPIN_NOTE, id),
            Self::KbConfirmNoteDeletion { id } => {
                write!(f, "{}@{}", cmd::KB_CONFIRM_NOTE_DELETION, id)
            }
            Self::KbCancelNoteDeletion { id } => {
                write!(f, "{}@{}", cmd::KB_CANCEL_NOTE_DELETION, id)
            }
            Self::KbEditDir { id } => {
                write!(f, "{}@{}", cmd::KB_EDIT_DIR, id)
            }
            Self::KbCreateNote { destination } => {
                write!(f, "{}@{}", cmd::KB_CREATE_NOTE, destination)
            }
            Self::KbMoveNoteHere { destination, note } => {
                write!(f, "{}@{},{}", cmd::KB_MOVE_NOTE_HERE, destination, note)
            }
            Self::KbMoveDirectoryHere {
                destination,
                directory,
            } => {
                write!(
                    f,
                    "{}@{},{}",
                    cmd::KB_MOVE_DIRECTORY_HERE,
                    destination,
                    directory
                )
            }
            Self::KbMoveDirectory { id } => {
                write!(f, "{}@{}", cmd::KB_MOVE_DIRECTORY, id)
            }
            Self::KbCreateDirectory { destination } => {
                write!(f, "{}@{}", cmd::KB_CREATE_DIR, destination)
            }
            Self::KbRenameDirectory { id } => {
                write!(f, "{}@{}", cmd::KB_RENAME_DIR, id)
            }
            Self::KbDeleteDirectory { id } => {
                write!(f, "{}@{}", cmd::KB_DELETE_DIR, id)
            }
            Self::KbPinDirectory { id } => {
                write!(f, "{}@{}", cmd::KB_PIN_DIR, id)
            }
            Self::KbUnpinDirectory { id } => {
                write!(f, "{}@{}", cmd::KB_UNPIN_DIR, id)
            }
            Self::KbConfirmDirectoryDeletion { id } => {
                write!(f, "{}@{}", cmd::KB_CONFIRM_DIR_DELETION, id)
            }
            Self::KbCancelDirectoryDeletion { id } => {
                write!(f, "{}@{}", cmd::KB_CANCEL_DIR_DELETION, id)
            }
            Self::FormOption { index } => write!(f, "{}@{}", cmd::FORM_OPTION, index),
            Self::Subscribe { newsletter } => write!(f, "{}@{}", cmd::SUBSCRIBE, &newsletter),
            Self::Unsubscribe { newsletter } => write!(f, "{}@{}", cmd::UNSUBSCRIBE, &newsletter),
            Self::ManageSubscriptions => write!(f, "{}", cmd::MANAGE_SUBSCRIPTIONS),
        }
    }
}

// Needed for seamless teloxide interoperation.
impl From<Query> for String {
    fn from(q: Query) -> Self {
        q.to_string()
    }
}

impl TryFrom<RawQuery<'_>> for Query {
    type Error = QueryParseError;

    fn try_from(value: RawQuery<'_>) -> Result<Self, Self::Error> {
        use strings::cmd;

        fn parse_id_pair<T, U>(s: Option<&str>) -> Option<(T, U)>
        where
            T: From<u64>,
            U: From<u64>,
        {
            let s = s?;
            let (left, right) = s.split_once(':')?;
            Some((
                left.parse::<u64>().ok()?.into(),
                right.parse::<u64>().ok()?.into(),
            ))
        }
        let err_fn = || QueryParseError::InvalidPayload {
            command: value.command.to_owned(),
            payload: value.payload.map(str::to_owned),
        };

        let parse_directory_id = |s| {
            let (provider, directory) = parse_id_pair(s).ok_or_else(err_fn)?;
            Ok(FullDirectoryId {
                provider,
                directory,
            })
        };

        let parse_note_id = |s| {
            let (provider, note) = parse_id_pair(s).ok_or_else(err_fn)?;
            Ok(FullNoteId { provider, note })
        };

        let parse_destination_note_pair = |s| {
            let s: &str = Option::ok_or_else(s, err_fn)?;
            let (left, right) = s.split_once(',').ok_or_else(err_fn)?;
            Ok((parse_directory_id(Some(left))?, parse_note_id(Some(right))?))
        };

        let parse_destination_dir_pair = |s| {
            let s: &str = Option::ok_or_else(s, err_fn)?;
            let (left, right) = s.split_once(',').ok_or_else(err_fn)?;
            Ok((
                parse_directory_id(Some(left))?,
                parse_directory_id(Some(right))?,
            ))
        };

        let (query, payload_must_be_none) = match value.command {
            cmd::OPEN_MAIN_MENU => (Query::OpenMainMenu, true),
            cmd::OPEN_KB => (Query::OpenKb, true),
            cmd::OPEN_NL_ARCHIVE => (Query::OpenNewsletterArchive, true),
            cmd::OPEN_CALENDAR => (Query::OpenCalendar, true),
            cmd::OPEN_FEEDBACK => (Query::OpenFeedback, true),
            cmd::OPEN_FEEDBACK_TOPIC => (
                Query::OpenFeedbackTopic {
                    topic: value.payload.and_then(|x| x.parse().ok()).ok_or_else(|| {
                        QueryParseError::InvalidPayload {
                            command: value.command.to_owned(),
                            payload: value.payload.map(str::to_owned),
                        }
                    })?,
                },
                false,
            ),
            cmd::KB_GO_UP => (Query::KbGoUp, true),
            cmd::KB_NAV_TO_DIR => (
                Query::KbNavToDir {
                    id: parse_directory_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_NAV_TO_NOTE => (
                Query::KbNavToNote {
                    id: parse_note_id(value.payload)?,
                },
                false,
            ),
            cmd::OPEN_NL_SETTINGS => (Query::OpenNlSettings, true),
            cmd::GO_BACK => (Query::GoBack, true),
            cmd::KB_EDIT_NOTE => (
                Query::KbEditNote {
                    id: parse_note_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_RENAME_NOTE => (
                Query::KbRenameNote {
                    id: parse_note_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_MOVE_NOTE => (
                Query::KbMoveNote {
                    id: parse_note_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_DELETE_NOTE => (
                Query::KbDeleteNote {
                    id: parse_note_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_PIN_NOTE => (
                Query::KbPinNote {
                    id: parse_note_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_UNPIN_NOTE => (
                Query::KbUnpinNote {
                    id: parse_note_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_CONFIRM_NOTE_DELETION => (
                Query::KbConfirmNoteDeletion {
                    id: parse_note_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_CANCEL_NOTE_DELETION => (
                Query::KbCancelNoteDeletion {
                    id: parse_note_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_EDIT_DIR => (
                Query::KbEditDir {
                    id: parse_directory_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_CREATE_NOTE => (
                Query::KbCreateNote {
                    destination: parse_directory_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_MOVE_NOTE_HERE => {
                let (destination, note) = parse_destination_note_pair(value.payload)?;
                (Query::KbMoveNoteHere { destination, note }, false)
            }
            cmd::KB_MOVE_DIRECTORY_HERE => {
                let (destination, directory) = parse_destination_dir_pair(value.payload)?;
                (
                    Query::KbMoveDirectoryHere {
                        destination,
                        directory,
                    },
                    false,
                )
            }
            cmd::KB_MOVE_DIRECTORY => (
                Query::KbMoveDirectory {
                    id: parse_directory_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_CREATE_DIR => (
                Query::KbCreateDirectory {
                    destination: parse_directory_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_RENAME_DIR => (
                Query::KbRenameDirectory {
                    id: parse_directory_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_DELETE_DIR => (
                Query::KbDeleteDirectory {
                    id: parse_directory_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_PIN_DIR => (
                Query::KbPinDirectory {
                    id: parse_directory_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_UNPIN_DIR => (
                Query::KbUnpinDirectory {
                    id: parse_directory_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_CONFIRM_DIR_DELETION => (
                Query::KbConfirmDirectoryDeletion {
                    id: parse_directory_id(value.payload)?,
                },
                false,
            ),
            cmd::KB_CANCEL_DIR_DELETION => (
                Query::KbCancelDirectoryDeletion {
                    id: parse_directory_id(value.payload)?,
                },
                false,
            ),
            cmd::FORM_OPTION => (
                Query::FormOption {
                    index: value.payload.and_then(|s| s.parse().ok()).ok_or_else(|| {
                        QueryParseError::InvalidPayload {
                            command: String::from(value.command),
                            payload: value.payload.map(|x| x.to_owned()),
                        }
                    })?,
                },
                false,
            ),
            cmd::SUBSCRIBE => (
                Query::Subscribe {
                    newsletter: value
                        .payload
                        .ok_or_else(|| QueryParseError::InvalidPayload {
                            command: value.command.to_owned(),
                            payload: None,
                        })?
                        .to_owned(),
                },
                false,
            ),
            cmd::UNSUBSCRIBE => (
                Query::Unsubscribe {
                    newsletter: value
                        .payload
                        .ok_or_else(|| QueryParseError::InvalidPayload {
                            command: value.command.to_owned(),
                            payload: None,
                        })?
                        .to_owned(),
                },
                false,
            ),
            cmd::MANAGE_SUBSCRIPTIONS => (Query::ManageSubscriptions, true),
            _ => {
                return Err(QueryParseError::InvalidCommand {
                    command: value.command.to_owned(),
                })
            }
        };

        if payload_must_be_none && !value.payload.is_none() {
            return Err(QueryParseError::InvalidPayload {
                command: value.command.to_owned(),
                payload: value.payload.map(str::to_owned),
            });
        }

        Ok(query)
    }
}

mod strings {
    pub mod cmd {
        pub const KB_GO_UP: &'static str = "kb-go-up";
        pub const KB_NAV_TO_DIR: &'static str = "kb-nav-to-dir";
        pub const KB_NAV_TO_NOTE: &'static str = "kb-nav-to-note";
        pub const OPEN_CALENDAR: &'static str = "open-calendar";
        pub const OPEN_FEEDBACK_TOPIC: &'static str = "open-feedback-topic";
        pub const OPEN_FEEDBACK: &'static str = "open-feedback";
        pub const OPEN_KB: &'static str = "open-kb";
        pub const OPEN_MAIN_MENU: &'static str = "open-main-menu";
        pub const OPEN_NL_ARCHIVE: &'static str = "open-nl-archive";
        pub const OPEN_NL_SETTINGS: &'static str = "open-nl-settings";
        pub const GO_BACK: &'static str = "kb-go-back";
        pub const KB_EDIT_NOTE: &'static str = "kb-edit-note";
        pub const KB_RENAME_NOTE: &'static str = "kb-rename-note";
        pub const KB_MOVE_NOTE: &'static str = "kb-move-note";
        pub const KB_DELETE_NOTE: &'static str = "kb-delete-note";
        pub const KB_PIN_NOTE: &'static str = "kb-pin-note";
        pub const KB_UNPIN_NOTE: &'static str = "kb-unpin-note";
        pub const KB_CONFIRM_NOTE_DELETION: &'static str = "kb-confirm-note-del";
        pub const KB_CANCEL_NOTE_DELETION: &'static str = "kb-cancel-note-del";
        pub const KB_EDIT_DIR: &'static str = "kb-edit-dir";
        pub const KB_CREATE_NOTE: &'static str = "kb-create-note";
        pub const KB_MOVE_NOTE_HERE: &'static str = "kb-move-note-here";
        pub const KB_MOVE_DIRECTORY_HERE: &'static str = "kb-move-dir-here";
        pub const KB_MOVE_DIRECTORY: &'static str = "kb-move-dir";
        pub const KB_CREATE_DIR: &'static str = "kb-create-dir";
        pub const KB_RENAME_DIR: &'static str = "kb-rename-dir";
        pub const KB_DELETE_DIR: &'static str = "kb-delete-dir";
        pub const KB_PIN_DIR: &'static str = "kb-pin-dir";
        pub const KB_UNPIN_DIR: &'static str = "kb-unpin-dir";
        pub const KB_CONFIRM_DIR_DELETION: &'static str = "kb-confirm-dir-del";
        pub const KB_CANCEL_DIR_DELETION: &'static str = "kb-cancel-dir-del";
        pub const FORM_OPTION: &'static str = "form-opt";
        pub const SUBSCRIBE: &'static str = "subscribe";
        pub const UNSUBSCRIBE: &'static str = "unsubscribe";
        pub const MANAGE_SUBSCRIPTIONS: &'static str = "open-sub-settings";
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct RawQuery<'a> {
    pub command: &'a str,
    pub payload: Option<&'a str>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum QueryParseError {
    InvalidSyntax,
    InvalidCommand {
        command: String,
    },
    InvalidPayload {
        command: String,
        payload: Option<String>,
    },
}

impl Display for QueryParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSyntax => write!(f, "Invalid callback query syntax"),
            Self::InvalidCommand { command } => {
                write!(f, "Invalid callback query command `{}`", &command)
            }
            Self::InvalidPayload { command, payload } => write!(
                f,
                "Invalid callback query payload `{:?}` for command `{}`",
                &payload, &command,
            ),
        }
    }
}

impl Error for QueryParseError {}

pub fn parse_callback_query<'a>(query: &'a str) -> Result<Query, QueryParseError> {
    lazy_static! {
        static ref REGEX: Regex =
            Regex::new(r"^(?P<command>[a-zA-Z0-9_-]+)(?:@(?P<payload>.*))?$").unwrap();
    }
    let captures = REGEX
        .captures(query)
        .ok_or(QueryParseError::InvalidSyntax)?;
    let command = captures.name("command").unwrap().as_str();
    let payload = captures.name("payload").map(|x| x.as_str());
    RawQuery { command, payload }.try_into()
}
