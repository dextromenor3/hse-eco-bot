use crate::message::{FormattedMessage, FormattedText};
use crate::state::DialogState;
use crate::strings::STRINGS;
use crate::types::{BotType, HandlerResult};
use crate::user::Permissions;
use crate::user::User;
use crate::user_facing_error::UserFacingError;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex, RwLock};
use teloxide::prelude::*;
use teloxide::requests::HasPayload;
use rusqlite::{Connection, params};

/// The dialog with a certain user.
pub struct UserDialog {
    chat_id: ChatId,
    data: RwLock<UserDialogData>,
}

/// The mutable data of a [`UserDialog`].
#[derive(Clone)]
pub struct UserDialogData {
    pub state: DialogState,
    pub user: User,
}

impl UserDialogData {
    /// Create the default user dialog data for the user with the given ID.
    pub fn new(user: User) -> Self {
        Self {
            state: Default::default(),
            user,
        }
    }
}

impl UserDialog {
    /// Create from the ID of the chat with the user and the ID of this user.
    pub fn new(chat_id: ChatId, user: User) -> Self {
        Self {
            chat_id,
            data: RwLock::new(UserDialogData::new(user)),
        }
    }

    pub fn chat_id(&self) -> ChatId {
        self.chat_id
    }

    pub async fn send_message_with_id(
        chat_id: ChatId,
        bot: &BotType,
        message: FormattedMessage,
    ) -> HandlerResult<()> {
        let mut request = bot.send_message(chat_id, message.text.raw_text);
        let payload = request.payload_mut();
        payload.entities = message.text.entities;
        payload.reply_markup = message.reply_markup;
        request.await?;
        Ok(())
    }

    /// Get the dialog data.
    pub fn data(&self) -> &RwLock<UserDialogData> {
        &self.data
    }

    /// Get the dialog data by a mutable reference.
    ///
    /// This method calls [`RwLock::get_mut`], so the lock is resolved statically without holding a
    /// guard.
    pub fn data_mut(&mut self) -> &mut UserDialogData {
        self.data.get_mut().unwrap()
    }
}

/// The error when the bot has been invoked in a kind of chat it does not support (e.g. in a group chat).
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct InvalidChatError {
    pub chat_id: ChatId,
}

impl std::fmt::Display for InvalidChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            concat!(
                "Chat with id {} is not a private chat. ",
                "The bot does not support group chats, channels or other non-private chats.",
            ),
            self.chat_id,
        )
    }
}

impl std::error::Error for InvalidChatError {}

impl UserFacingError for InvalidChatError {
    fn user_message(&self) -> FormattedText {
        STRINGS.errors.common.invalid_chat()
    }
}

/// The implementation that stores the information about dialogs and allows it to be retrieved or
/// modified in a thread-safe way.
pub struct DialogStorage {
    raw: Mutex<RefCell<RawDialogStorage>>,
}

pub struct RawDialogStorage {
    dialogs: HashMap<UserId, Arc<UserDialog>>,
    dialogs_by_username: HashMap<String, UserDialogData>,
}

impl DialogStorage {
    /// Create an empty [`DialogStorage`].
    pub fn new(db: &Connection) -> Self {
        let mut dialogs_by_username = HashMap::new();

        let txn = db.unchecked_transaction().unwrap();
        let mut stmt = txn.prepare("SELECT user, edit_kb, receive_feedback FROM permissions").unwrap();
        let permissions_for_users = stmt.query_map(params![], |row| {
            let user: String = row.get(0)?;
            let edit_kb = row.get(1)?;
            let receive_feedback = row.get(2)?;
            Ok((user, Permissions { edit_kb, receive_feedback, ..Default::default() }))
        }).unwrap();

        for (username, permissions) in permissions_for_users.map(|x| x.unwrap()) {
            debug!("Granting @{} with additional permissions", &username);
            let mut user = User::new();
            *user.permissions_mut() = permissions;
            if permissions.receive_feedback {
                user.subscriptions_mut().insert(String::from("feedback"));
            }
            let dialog_data = UserDialogData::new(user);
            dialogs_by_username.insert(username, dialog_data);
        }

        Self {
            raw: Mutex::new(RefCell::new(RawDialogStorage {
                dialogs: HashMap::new(),
                dialogs_by_username,
            })),
        }
    }

    /// Get the dialog with the specified chat and user IDs.
    ///
    /// The dialog can be modified through the returned smart pointer, and the changes will be
    /// reflected in the [`DialogStorage`]. The lock on the dialog is independent from the lock on the
    /// dialog map, so possession of the returned smart pointer does not affect other [`get_dialog`]
    /// operations in any way.
    ///
    /// Both chat and user IDs are needed to ensure that both the chat and the user stay the same
    /// throughout the dialog. Also, it might prove necessary to filter out group chats, which
    /// our bot does not support.
    pub fn get_dialog(
        &self,
        chat_id: ChatId,
        user_id: UserId,
        maybe_username: Option<&str>,
    ) -> Result<Arc<UserDialog>, InvalidChatError> {
        if !chat_id.is_user() {
            return Err(InvalidChatError { chat_id });
        }

        let lock = self.raw.lock().unwrap();
        if let Some(username) = maybe_username {
            let contains_key = lock.borrow().dialogs_by_username.contains_key(username);
            if contains_key {
                debug!("Recognizing @{}", username);
                let mut borrow_mut = lock.borrow_mut();
                let dialog_data = borrow_mut.dialogs_by_username.remove(username).unwrap();
                let dialog = UserDialog {
                    chat_id,
                    data: RwLock::new(dialog_data),
                };
                borrow_mut.dialogs.insert(user_id, Arc::new(dialog));
            }
        }

        let mut borrow_mut = lock.borrow_mut();
        let dialog_ref = borrow_mut.dialogs.entry(user_id).or_insert_with(|| {
            let user = User::new();
            let dialog = UserDialog::new(chat_id, user);
            Arc::new(dialog)
        });
        Ok(Arc::clone(dialog_ref))
    }

    pub fn inspect_dialogs<F>(&self, inspector: &mut F)
    where
        F: FnMut(UserId, &Arc<UserDialog>),
    {
        for (&k, ref v) in self.raw.lock().unwrap().borrow().dialogs.iter() {
            inspector(k, v);
        }
    }
}
