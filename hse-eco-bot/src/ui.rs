pub mod form;

use crate::callback_query::{parse_callback_query, Query};
use crate::db::{FullDirectoryId, FullItemId, FullNoteId};
use crate::dispatch::UserDialog;
use crate::feedback::FeedbackTopic;
use crate::global_state::GlobalState;
use crate::invalid_action::InvalidAction;
use crate::kb::{Note, ProviderError, ProviderUserContext};
use crate::media::Location;
use crate::message::{FormattedMessage, FormattedText};
use crate::message_format_error::MessageFormatError;
use crate::message_queue::MessageQueueSender;
use crate::state::{states, DialogState};
use crate::strings::STRINGS;
use crate::types::{BotType, HandlerError, HandlerResult};
use crate::user_facing_error::UserFacingError;
use form::{Form, FormElement, FormFillingState, FormInputType, FormRawInput};
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{
    ButtonRequest, InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton, KeyboardMarkup,
    MediaKind, MessageKind,
};

fn is_name_valid(name: &str) -> bool {
    name.find(&['\0', '/', '\\']).is_none()
}

fn extract_name(message: &Message) -> Result<&str, MessageFormatError> {
    let name = match message.text() {
        Some(text) => text,
        None => return Err(MessageFormatError::NoText.into()),
    };

    let has_attachments = match &message.kind {
        MessageKind::Common(common) => match common.media_kind {
            MediaKind::Text(_) => false,
            _ => true,
        },
        _ => true,
    };
    if has_attachments {
        return Err(MessageFormatError::HasAttachments.into());
    }

    if !is_name_valid(name) {
        return Err(MessageFormatError::InvalidName.into());
    }

    Ok(name)
}

fn extract_formatted_text(message: &Message) -> Result<FormattedText, MessageFormatError> {
    let raw_text = match message.text().or_else(|| message.caption()) {
        Some(text) => text.to_owned(),
        None => return Err(MessageFormatError::NoText.into()),
    };
    let entities = message
        .entities()
        .or_else(|| message.caption_entities())
        .map(|x| x.to_owned());
    let text = FormattedText { raw_text, entities };
    Ok(text)
}

struct Context<'bot, 'dialog, 'gs, 'mq> {
    pub bot: &'bot BotType,
    pub dialog: &'dialog UserDialog,
    pub global_state: &'gs Arc<GlobalState>,
    pub message_queue_tx: &'mq mut MessageQueueSender,
}

/// Handle an incoming message in the initial state.
pub async fn handle_message(
    bot: BotType,
    message: Message,
    global_state: Arc<GlobalState>,
    mut message_queue_tx: MessageQueueSender,
) -> HandlerResult<()> {
    let (user_id, maybe_username) = match message.from() {
        Some(ref user) => (user.id, user.username.as_deref()),
        None => {
            // Ignore messages from an unknown sender or without a sender.
            return Ok(());
        }
    };

    let dialog =
        global_state
            .dialog_storage
            .get_dialog(message.chat.id, user_id, maybe_username)?;
    let state = dialog.data().read().unwrap().state.clone();

    let mut context = Context {
        bot: &bot,
        dialog: &dialog,
        global_state: &global_state,
        message_queue_tx: &mut message_queue_tx,
    };

    let result = match state {
        DialogState::Initial => context.handle_initial_message(message).await,
        DialogState::MainMenu => context.handle_main_menu_message(message).await,
        DialogState::KbNavigation(_) => Err(InvalidAction::UnexpectedMessage.into()),
        DialogState::KbNoteViewing(_) => Err(InvalidAction::UnexpectedMessage.into()),
        DialogState::KbNoteDeletionConfirmation(_) => Err(InvalidAction::UnexpectedMessage.into()),
        DialogState::KbNoteRenaming(state_struct) => {
            context
                .handle_note_renaming_message(message, state_struct)
                .await
        }
        DialogState::KbNoteCreation(state_struct) => {
            context
                .handle_note_creation_message(message, state_struct)
                .await
        }
        DialogState::KbNoteCreationNamed(state_struct) => {
            context
                .handle_note_creation_named_message(message, state_struct)
                .await
        }
        DialogState::KbDirectoryEditing(_) => Err(InvalidAction::UnexpectedMessage.into()),
        DialogState::KbNoteEditing(state_struct) => {
            context
                .handle_note_editing_message(message, state_struct)
                .await
        }
        DialogState::KbNoteMovement(_) => Err(InvalidAction::UnexpectedMessage.into()),
        DialogState::KbDirectoryMovement(_) => Err(InvalidAction::UnexpectedMessage.into()),
        DialogState::KbDirectoryCreation(state_struct) => {
            context
                .handle_directory_creation_message(message, state_struct)
                .await
        }
        DialogState::KbDirectoryRenaming(state_struct) => {
            context
                .handle_directory_renaming_message(message, state_struct)
                .await
        }
        DialogState::KbDirectoryDeletion(_) => Err(InvalidAction::UnexpectedMessage.into()),
        DialogState::FeedbackTopicSelection => Err(InvalidAction::UnexpectedMessage.into()),
        DialogState::FormFilling(state_struct) => {
            context
                .handle_form_filling_message(message, state_struct)
                .await
        }
        DialogState::SubscriptionsMenu => Err(InvalidAction::UnexpectedMessage.into()),
    };

    match result {
        Ok(()) => Ok(()),
        Err(HandlerError::Internal(e)) => Err(e.into()),
        Err(HandlerError::User(e)) => {
            debug!("User error: {:?}", &e);
            context
                .send_message(FormattedMessage::new(e.user_message()))
                .await?;
            context.send_state_prompt().await?;
            Ok(())
        }
    }
}

/// Handle an incoming callback query.
pub async fn handle_callback_query(
    bot: BotType,
    query: CallbackQuery,
    global_state: Arc<GlobalState>,
    mut message_queue_tx: MessageQueueSender,
) -> HandlerResult<()> {
    let query_data = match query.data {
        Some(data) => data,
        None => return Ok(()),
    };
    let chat_id = match query.message {
        Some(message) => message.chat.id,
        None => return Ok(()),
    };
    let user_id = query.from.id;
    let maybe_username = query.from.username.as_deref();

    let dialog = global_state
        .dialog_storage
        .get_dialog(chat_id, user_id, maybe_username)?;

    let mut context = Context {
        bot: &bot,
        dialog: &dialog,
        global_state: &global_state,
        message_queue_tx: &mut message_queue_tx,
    };

    bot.answer_callback_query(query.id).await?;

    // Save match result into a temporary variable to drop the lock before the next `await`.
    let is_initial = if let DialogState::Initial = dialog.data().read().unwrap().state {
        true
    } else {
        false
    };
    if is_initial {
        context
            .send_message(FormattedMessage::new(STRINGS.initial.invalid_action()))
            .await?;
        return Ok(());
    }

    let parsed_query = match parse_callback_query(&query_data) {
        Ok(parsed_query) => parsed_query,
        Err(e) => {
            warn!("Invalid callback query: {}", e);
            context
                .send_message(FormattedMessage::new(
                    STRINGS.technical.invalid_callback_query(),
                ))
                .await?;
            return Ok(());
        }
    };

    let result = context.handle_callback_query(&parsed_query).await;

    match result {
        Ok(()) => Ok(()),
        Err(HandlerError::Internal(e)) => Err(e.into()),
        Err(HandlerError::User(e)) => {
            debug!("User error: {:?}", &e);
            context
                .send_message(FormattedMessage::new(e.user_message()))
                .await?;
            context.send_state_prompt().await?;
            Ok(())
        }
    }
}

impl Context<'_, '_, '_, '_> {
    async fn send_message(&mut self, message: FormattedMessage) -> HandlerResult<()> {
        self.message_queue_tx
            .send_message(message, self.dialog.chat_id())
            .await
    }

    fn set_state(&self, new_state: DialogState) {
        self.dialog.data().write().unwrap().state = new_state;
    }

    fn state(&self) -> DialogState {
        self.dialog.data().read().unwrap().state.clone()
    }

    async fn send_todo(&mut self, text: &str) -> HandlerResult<()> {
        self.send_message(FormattedMessage::new(STRINGS.technical.todo(text)))
            .await?;
        Ok(())
    }

    fn uctx(&self) -> ProviderUserContext {
        ProviderUserContext {
            permissions: *self.dialog.data().read().unwrap().user.permissions(),
        }
    }

    async fn handle_callback_query(&mut self, query: &Query) -> HandlerResult<()> {
        let uctx = self.uctx();
        match query {
            Query::OpenMainMenu => self.set_state(DialogState::MainMenu),
            Query::OpenKb => {
                let id = self.global_state.db.root_directory(uctx).await?;
                self.set_state(DialogState::KbNavigation(states::KbNavigation { id }));
            }
            Query::OpenNewsletterArchive => {
                let root_id = self.global_state.db.root_directory(uctx).await?;
                let archive_id = self
                    .global_state
                    .db
                    .read_directory(uctx, root_id)
                    .await?
                    .directories
                    .into_iter()
                    .find(|&(ref name, _)| name == "Архив рассылок")
                    .unwrap()
                    .1;
                self.set_state(DialogState::KbNavigation(states::KbNavigation {
                    id: archive_id,
                }));
            }
            Query::OpenCalendar => {
                self.send_todo("Open calendar").await?;
            }
            Query::OpenFeedback => {
                self.set_state(DialogState::FeedbackTopicSelection);
            }
            Query::OpenFeedbackTopic { topic } => {
                self.start_feedback_form_filling(*topic);
            }
            Query::KbGoUp => {
                let db = &self.global_state.db;
                match self.state() {
                    DialogState::KbNavigation(nav) => {
                        let maybe_parent = db.directory_parent(uctx, nav.id).await?;
                        match maybe_parent {
                            Some(parent) => {
                                self.set_state(DialogState::KbNavigation(states::KbNavigation {
                                    id: parent,
                                }));
                            }
                            None => {
                                return Err(InvalidAction::CannotGoUp.into());
                            }
                        }
                    }
                    DialogState::KbNoteMovement(mv) => {
                        let maybe_parent = db.directory_parent(uctx, mv.destination).await?;
                        match maybe_parent {
                            Some(parent) => {
                                self.set_state(DialogState::KbNoteMovement(
                                    states::KbNoteMovement {
                                        destination: parent,
                                        note: mv.note,
                                    },
                                ));
                            }
                            None => {
                                return Err(InvalidAction::CannotGoUp.into());
                            }
                        }
                    }
                    DialogState::KbDirectoryMovement(mv) => {
                        let maybe_parent = db.directory_parent(uctx, mv.destination).await?;
                        match maybe_parent {
                            Some(parent) => {
                                self.set_state(DialogState::KbDirectoryMovement(
                                    states::KbDirectoryMovement {
                                        destination: parent,
                                        directory: mv.directory,
                                    },
                                ));
                            }
                            None => {
                                return Err(InvalidAction::CannotGoUp.into());
                            }
                        }
                    }
                    _ => return Err(InvalidAction::InvalidState.into()),
                }
            }
            Query::KbNavToDir { id } => match self.state() {
                DialogState::KbNoteMovement(mv) => {
                    self.set_state(DialogState::KbNoteMovement(states::KbNoteMovement {
                        destination: *id,
                        note: mv.note,
                    }));
                }
                DialogState::KbDirectoryMovement(mv) => {
                    self.set_state(DialogState::KbDirectoryMovement(
                        states::KbDirectoryMovement {
                            destination: *id,
                            directory: mv.directory,
                        },
                    ));
                }
                _ => self.set_state(DialogState::KbNavigation(states::KbNavigation { id: *id })),
            },
            Query::KbNavToNote { id } => {
                self.set_state(DialogState::KbNoteViewing(states::KbNoteViewing {
                    id: *id,
                }));
            }
            Query::OpenNlSettings => self.set_state(DialogState::SubscriptionsMenu),
            Query::GoBack => {
                let db = &self.global_state.db;
                match self.state() {
                    DialogState::KbNoteViewing(view) => {
                        let parent = db.note_parent(uctx, view.id).await?;
                        self.set_state(DialogState::KbNavigation(states::KbNavigation {
                            id: parent,
                        }));
                    }
                    DialogState::KbNoteRenaming(ren) => {
                        let parent = db.note_parent(uctx, ren.id).await?;
                        self.set_state(DialogState::KbNavigation(states::KbNavigation {
                            id: parent,
                        }));
                    }
                    DialogState::KbNoteCreation(cre) => {
                        self.set_state(DialogState::KbNavigation(states::KbNavigation {
                            id: cre.destination,
                        }));
                    }
                    DialogState::KbNoteCreationNamed(cre) => {
                        self.set_state(DialogState::KbNoteCreation(states::KbNoteCreation {
                            destination: cre.destination,
                        }));
                    }
                    DialogState::KbDirectoryEditing(edit) => {
                        self.set_state(DialogState::KbNavigation(states::KbNavigation {
                            id: edit.id,
                        }));
                    }
                    DialogState::KbNoteEditing(edit) => {
                        self.set_state(DialogState::KbNoteViewing(states::KbNoteViewing {
                            id: edit.id,
                        }));
                    }
                    DialogState::KbNoteMovement(mv) => {
                        self.set_state(DialogState::KbNoteViewing(states::KbNoteViewing {
                            id: mv.note,
                        }));
                    }
                    DialogState::KbDirectoryMovement(mv) => {
                        self.set_state(DialogState::KbNavigation(states::KbNavigation {
                            id: mv.directory,
                        }));
                    }
                    DialogState::KbDirectoryCreation(cre) => {
                        self.set_state(DialogState::KbNavigation(states::KbNavigation {
                            id: cre.destination,
                        }));
                    }
                    DialogState::KbDirectoryRenaming(ren) => {
                        self.set_state(DialogState::KbNavigation(states::KbNavigation {
                            id: ren.id,
                        }));
                    }
                    DialogState::FormFilling(mut fill) => {
                        if !fill.form_state.can_go_back() {
                            return Err(InvalidAction::InvalidState.into());
                        }
                        fill.form_state.back();
                        self.set_state(DialogState::FormFilling(fill));
                    }
                    DialogState::SubscriptionsMenu => {
                        self.set_state(DialogState::MainMenu);
                    }
                    _ => return Err(InvalidAction::InvalidState.into()),
                }
            }
            Query::KbEditNote { id } => {
                self.set_state(DialogState::KbNoteEditing(states::KbNoteEditing {
                    id: *id,
                }))
            }
            Query::KbRenameNote { id } => {
                self.set_state(DialogState::KbNoteRenaming(states::KbNoteRenaming {
                    id: *id,
                }));
            }
            Query::KbMoveNote { id } => {
                let dir = self.global_state.db.note_parent(uctx, *id).await?;
                self.set_state(DialogState::KbNoteMovement(states::KbNoteMovement {
                    destination: dir,
                    note: *id,
                }));
            }
            Query::KbDeleteNote { id } => match self.state() {
                DialogState::KbNoteViewing(view) if view.id == *id => {
                    self.set_state(DialogState::KbNoteDeletionConfirmation(
                        states::KbNoteDeletionConfirmation { id: *id },
                    ));
                }
                _ => return Err(InvalidAction::InvalidState.into()),
            },
            Query::KbPinNote { id } => self.send_todo(&format!("Pin note {}", id)).await?,
            Query::KbUnpinNote { id } => self.send_todo(&format!("Unpin note {}", id)).await?,
            Query::KbConfirmNoteDeletion { id } => {
                let db = &self.global_state.db;
                match self.state() {
                    DialogState::KbNoteDeletionConfirmation(confirmation)
                        if confirmation.id == *id =>
                    {
                        self.require_kb_edit_permission()?;
                        let parent = db.note_parent(uctx, *id).await?;
                        self.set_state(DialogState::KbNavigation(states::KbNavigation {
                            id: parent,
                        }));
                        db.delete_note(uctx, *id).await?;
                    }
                    _ => return Err(InvalidAction::InvalidState.into()),
                }
            }
            Query::KbCancelNoteDeletion { id } => match self.state() {
                DialogState::KbNoteDeletionConfirmation(confirmation) if confirmation.id == *id => {
                    self.set_state(DialogState::KbNoteViewing(states::KbNoteViewing {
                        id: *id,
                    }));
                }
                _ => return Err(InvalidAction::InvalidState.into()),
            },
            Query::KbEditDir { id } => match self.state() {
                DialogState::KbNavigation(nav) if nav.id == *id => self.set_state(
                    DialogState::KbDirectoryEditing(states::KbDirectoryEditing { id: *id }),
                ),
                _ => return Err(InvalidAction::InvalidState.into()),
            },
            Query::KbCreateNote { destination } => match self.state() {
                DialogState::KbDirectoryEditing(edit) if edit.id == *destination => {
                    self.set_state(DialogState::KbNoteCreation(states::KbNoteCreation {
                        destination: *destination,
                    }));
                }
                _ => return Err(InvalidAction::InvalidState.into()),
            },
            Query::KbMoveNoteHere { note, destination } => match self.state() {
                DialogState::KbNoteMovement(mv)
                    if mv.note == *note && mv.destination == *destination =>
                {
                    self.require_kb_edit_permission()?;
                    self.global_state
                        .db
                        .move_note(uctx, mv.note, mv.destination)
                        .await?;
                    self.set_state(DialogState::KbNoteViewing(states::KbNoteViewing {
                        id: mv.note,
                    }));
                }
                _ => return Err(InvalidAction::InvalidState.into()),
            },
            Query::KbMoveDirectoryHere {
                directory,
                destination,
            } => match self.state() {
                DialogState::KbDirectoryMovement(mv)
                    if mv.directory == *directory && mv.destination == *destination =>
                {
                    self.require_kb_edit_permission()?;
                    self.global_state
                        .db
                        .move_directory(uctx, mv.directory, mv.destination)
                        .await?;
                    self.set_state(DialogState::KbNavigation(states::KbNavigation {
                        id: mv.directory,
                    }));
                }
                _ => return Err(InvalidAction::InvalidState.into()),
            },
            Query::KbMoveDirectory { id } => {
                self.require_kb_edit_permission()?;
                let dir = self
                    .global_state
                    .db
                    .directory_parent(uctx, *id)
                    .await?
                    .ok_or(ProviderError::CannotMoveRoot)?;
                self.set_state(DialogState::KbDirectoryMovement(
                    states::KbDirectoryMovement {
                        directory: *id,
                        destination: dir,
                    },
                ));
            }
            Query::KbCreateDirectory { destination } => {
                self.set_state(DialogState::KbDirectoryCreation(
                    states::KbDirectoryCreation {
                        destination: *destination,
                    },
                ));
            }
            Query::KbRenameDirectory { id } => {
                self.require_kb_edit_permission()?;
                if self
                    .global_state
                    .db
                    .directory_parent(uctx, *id)
                    .await?
                    .is_none()
                {
                    return Err(ProviderError::CannotRenameRoot.into());
                }
                self.set_state(DialogState::KbDirectoryRenaming(
                    states::KbDirectoryRenaming { id: *id },
                ));
            }
            Query::KbDeleteDirectory { id } => {
                if self
                    .global_state
                    .db
                    .directory_parent(uctx, *id)
                    .await?
                    .is_none()
                {
                    return Err(ProviderError::CannotDeleteRoot.into());
                }
                self.set_state(DialogState::KbDirectoryDeletion(
                    states::KbDirectoryDeletion { id: *id },
                ));
            }
            Query::KbPinDirectory { id } => {
                self.send_todo(&format!("pin directory {}", id)).await?;
            }
            Query::KbUnpinDirectory { id } => {
                self.send_todo(&format!("unpin directory {}", id)).await?;
            }
            Query::KbConfirmDirectoryDeletion { id } => match self.state() {
                DialogState::KbDirectoryDeletion(del) if del.id == *id => {
                    self.require_kb_edit_permission()?;
                    let parent = self
                        .global_state
                        .db
                        .directory_parent(uctx, *id)
                        .await?
                        .ok_or(ProviderError::CannotDeleteRoot)?;
                    self.global_state.db.delete_directory(uctx, *id).await?;
                    self.set_state(DialogState::KbNavigation(states::KbNavigation {
                        id: parent,
                    }));
                }
                _ => return Err(InvalidAction::InvalidState.into()),
            },
            Query::KbCancelDirectoryDeletion { id } => match self.state() {
                DialogState::KbDirectoryDeletion(del) if del.id == *id => {
                    self.set_state(DialogState::KbNavigation(states::KbNavigation { id: *id }));
                }
                _ => return Err(InvalidAction::InvalidState.into()),
            },
            Query::FormOption { index } => match self.state() {
                DialogState::FormFilling(mut fill) => {
                    fill.form_state
                        .next(FormRawInput::Choice { index: *index })?;
                    if fill.form_state.is_done() {
                        self.set_state(*fill.completion_state);
                    } else {
                        self.set_state(DialogState::FormFilling(fill));
                    }
                }
                _ => return Err(InvalidAction::InvalidState.into()),
            },
            Query::Subscribe { ref newsletter } => {
                let ok = {
                    let mut dialog_data = self.dialog.data().write().unwrap();
                    let subscriptions = dialog_data.user.subscriptions_mut();

                    if subscriptions.contains(newsletter) {
                        false
                    } else {
                        subscriptions.insert(newsletter.clone());
                        true
                    }
                };

                if ok {
                    self.send_message(STRINGS.newsletter.subscribed().into())
                        .await?;
                } else {
                    self.send_message(STRINGS.newsletter.already_subscribed().into())
                        .await?;
                }
            }
            Query::Unsubscribe { ref newsletter } => {
                let ok = {
                    let mut dialog_data = self.dialog.data().write().unwrap();
                    let subscriptions = dialog_data.user.subscriptions_mut();

                    if subscriptions.contains(newsletter) {
                        subscriptions.remove(newsletter);
                        true
                    } else {
                        false
                    }
                };

                if ok {
                    self.send_message(STRINGS.newsletter.unsubscribed().into())
                        .await?;
                } else {
                    self.send_message(STRINGS.newsletter.not_subscribed().into())
                        .await?;
                }
            }
            Query::ManageSubscriptions => {
                self.set_state(DialogState::SubscriptionsMenu);
            }
        };
        self.send_state_prompt().await?;

        Ok(())
    }

    async fn send_state_prompt(&mut self) -> HandlerResult<()> {
        let state = self.dialog.data().read().unwrap().state.clone();
        match state {
            DialogState::Initial => Ok(()),
            DialogState::MainMenu => self.send_main_menu().await,
            DialogState::KbNavigation(nav) => self.send_kb_directory(nav.id, None).await,
            DialogState::KbNoteViewing(view) => self.send_kb_note(view.id).await,
            DialogState::KbNoteDeletionConfirmation(confirmation) => {
                self.send_note_deletion_confirmation(confirmation.id).await
            }
            DialogState::KbNoteRenaming(ren) => self.send_note_renaming_prompt(ren.id).await,
            DialogState::KbNoteCreation(_) => self.send_note_creation_prompt().await,
            DialogState::KbNoteCreationNamed(_) => self.send_note_creation_named_prompt().await,
            DialogState::KbDirectoryEditing(edit) => {
                self.send_directory_editing_prompt(edit.id).await
            }
            DialogState::KbNoteEditing(edit) => self.send_note_editing_prompt(edit.id).await,
            DialogState::KbNoteMovement(mv) => {
                self.send_kb_directory(mv.destination, Some(FullItemId::Note(mv.note)))
                    .await
            }
            DialogState::KbDirectoryMovement(mv) => {
                self.send_kb_directory(mv.destination, Some(FullItemId::Directory(mv.directory)))
                    .await
            }
            DialogState::KbDirectoryCreation(_) => self.send_directory_creation_prompt().await,
            DialogState::KbDirectoryRenaming(ren) => {
                self.send_directory_renaming_prompt(ren.id).await
            }
            DialogState::KbDirectoryDeletion(del) => {
                self.send_directory_deletion_confirmation(del.id).await
            }
            DialogState::FeedbackTopicSelection => self.send_feedback_prompt().await,
            DialogState::FormFilling(fill) => self.send_form_filling_prompt(fill).await,
            DialogState::SubscriptionsMenu => self.send_subscriptions_menu().await,
        }
    }

    async fn handle_initial_message(&mut self, message: Message) -> HandlerResult<()> {
        let has_attachments = match message.kind {
            MessageKind::Common(common) => match common.media_kind {
                MediaKind::Text(_) => false,
                _ => true,
            },
            _ => true,
        };
        if has_attachments {
            self.send_message(STRINGS.initial.message_has_attachments().into())
                .await?;
            return Ok(());
        }

        trace!("Sending welcome message");
        self.send_message(STRINGS.initial.welcome().into()).await?;
        self.set_state(DialogState::MainMenu);
        self.send_main_menu().await?;

        Ok(())
    }

    async fn handle_main_menu_message(&mut self, _message: Message) -> HandlerResult<()> {
        self.send_message(STRINGS.main_menu.invalid_action().into())
            .await?;
        self.send_main_menu().await?;
        Ok(())
    }

    async fn handle_note_renaming_message(
        &mut self,
        message: Message,
        state: states::KbNoteRenaming,
    ) -> HandlerResult<()> {
        let new_name = extract_name(&message)?;
        self.require_kb_edit_permission()?;

        self.global_state
            .db
            .rename_note(self.uctx(), state.id, new_name.to_owned())
            .await?;

        self.send_message(FormattedMessage::new(STRINGS.kb.note_renaming_ok(new_name)))
            .await?;
        self.set_state(DialogState::KbNoteViewing(states::KbNoteViewing {
            id: state.id,
        }));
        self.send_state_prompt().await?;
        Ok(())
    }

    async fn handle_note_creation_message(
        &mut self,
        message: Message,
        state: states::KbNoteCreation,
    ) -> HandlerResult<()> {
        let name = extract_name(&message)?;
        self.send_note_creation_named_prompt().await?;

        self.set_state(DialogState::KbNoteCreationNamed(
            states::KbNoteCreationNamed {
                destination: state.destination,
                name: name.to_owned(),
            },
        ));
        Ok(())
    }

    async fn handle_note_creation_named_message(
        &mut self,
        message: Message,
        state: states::KbNoteCreationNamed,
    ) -> HandlerResult<()> {
        // TODO: save attachments.
        let note = Note {
            text: extract_formatted_text(&message)?,
        };
        self.require_kb_edit_permission()?;

        self.global_state
            .db
            .create_note(self.uctx(), state.destination, state.name.clone(), note)
            .await?;

        self.set_state(DialogState::KbNavigation(states::KbNavigation {
            id: state.destination,
        }));

        self.send_message(STRINGS.kb.note_creation_ok(&state.name).into())
            .await?;
        self.send_state_prompt().await?;
        Ok(())
    }

    async fn handle_note_editing_message(
        &mut self,
        message: Message,
        state: states::KbNoteEditing,
    ) -> HandlerResult<()> {
        // TODO: save attachments.
        let note = Note {
            text: extract_formatted_text(&message)?,
        };
        self.require_kb_edit_permission()?;

        let uctx = self.uctx();
        self.global_state
            .db
            .update_note(uctx, state.id, note)
            .await?;
        let parent = self.global_state.db.note_parent(uctx, state.id).await?;
        let note_name = self.global_state.db.note_name(uctx, state.id).await?;

        self.set_state(DialogState::KbNavigation(states::KbNavigation {
            id: parent,
        }));

        self.send_message(STRINGS.kb.note_editing_ok(&note_name).into())
            .await?;
        self.send_state_prompt().await?;
        Ok(())
    }

    async fn handle_directory_creation_message(
        &mut self,
        message: Message,
        state: states::KbDirectoryCreation,
    ) -> HandlerResult<()> {
        let name = extract_name(&message)?;
        self.require_kb_edit_permission()?;
        self.global_state
            .db
            .create_directory(self.uctx(), state.destination, name.to_owned())
            .await?;

        self.send_message(STRINGS.kb.directory_creation_ok(name).into())
            .await?;

        self.set_state(DialogState::KbNavigation(states::KbNavigation {
            id: state.destination,
        }));
        self.send_state_prompt().await?;
        Ok(())
    }

    async fn handle_directory_renaming_message(
        &mut self,
        message: Message,
        state: states::KbDirectoryRenaming,
    ) -> HandlerResult<()> {
        let name = extract_name(&message)?;
        self.require_kb_edit_permission()?;
        self.global_state
            .db
            .rename_directory(self.uctx(), state.id, name.to_owned())
            .await?;

        self.send_message(STRINGS.kb.directory_renaming_ok().into())
            .await?;
        self.set_state(DialogState::KbNavigation(states::KbNavigation {
            id: state.id,
        }));
        self.send_state_prompt().await?;

        Ok(())
    }

    async fn handle_form_filling_message(
        &mut self,
        message: Message,
        mut state: states::FormFilling,
    ) -> HandlerResult<()> {
        let message_common = if let MessageKind::Common(c) = message.kind {
            c
        } else {
            return Err(InvalidAction::UnexpectedMessageKind.into());
        };

        let raw_input = match message_common.media_kind {
            MediaKind::Text(text) => {
                if text.entities.is_empty() {
                    FormRawInput::Text { text: text.text }
                } else {
                    FormRawInput::FormattedText {
                        text: FormattedText {
                            raw_text: text.text,
                            entities: Some(text.entities),
                        },
                    }
                }
            }
            MediaKind::Location(loc) => FormRawInput::Location {
                location: Location {
                    latitude: loc.location.latitude,
                    longitude: loc.location.longitude,
                    accuracy: loc.location.horizontal_accuracy,
                },
            },
            MediaKind::Photo(photo) => FormRawInput::Message {
                // TODO: attachments.
                message: FormattedMessage::new(FormattedText {
                    raw_text: photo.caption.unwrap_or_default(),
                    entities: Some(photo.caption_entities),
                }),
            },
            MediaKind::Document(doc) => FormRawInput::Message {
                // TODO: attachments.
                message: FormattedMessage::new(FormattedText {
                    raw_text: doc.caption.unwrap_or_default(),
                    entities: Some(doc.caption_entities),
                }),
            },
            MediaKind::Video(video) => FormRawInput::Message {
                // TODO: attachments.
                message: FormattedMessage::new(FormattedText {
                    raw_text: video.caption.unwrap_or_default(),
                    entities: Some(video.caption_entities),
                }),
            },
            _ => return Err(InvalidAction::UnexpectedMessageKind.into()),
        };

        state.form_state.next(raw_input)?;
        if state.form_state.is_done() {
            state
                .on_completion
                .send(state.form_state.into_parts())
                .await
                .unwrap();
            self.set_state(*state.completion_state);
            self.send_message(STRINGS.form.complete().into()).await?;
        } else {
            self.set_state(DialogState::FormFilling(state));
        }
        self.send_state_prompt().await?;
        Ok(())
    }

    fn require_kb_edit_permission(&mut self) -> Result<(), ProviderError> {
        if self
            .dialog
            .data()
            .read()
            .unwrap()
            .user
            .permissions()
            .edit_kb
        {
            Ok(())
        } else {
            Err(ProviderError::PermissionDenied)
        }
    }

    /// Send the main menu to the user.
    async fn send_main_menu(&mut self) -> HandlerResult<()> {
        trace!("Sending main menu");
        let messages = [
            FormattedMessage::with_markup(
                STRINGS.main_menu.header1(),
                InlineKeyboardMarkup {
                    inline_keyboard: vec![
                        vec![InlineKeyboardButton::callback(
                            "📂 Все заметки",
                            Query::OpenKb,
                        )],
                        vec![InlineKeyboardButton::callback(
                            "🗂 Архив рассылок",
                            Query::OpenNewsletterArchive,
                        )],
                    ],
                }
                .into(),
            ),
            //FormattedMessage::with_markup(
            //    STRINGS.main_menu.header2(),
            //    InlineKeyboardMarkup {
            //        inline_keyboard: vec![vec![InlineKeyboardButton::callback(
            //            "📅 Календарь событий",
            //            Query::OpenCalendar,
            //        )]],
            //    }
            //    .into(),
            //),
            FormattedMessage::with_markup(
                STRINGS.main_menu.header3(),
                InlineKeyboardMarkup {
                    inline_keyboard: vec![
                        vec![InlineKeyboardButton::callback(
                            "🗞 Управление рассылками",
                            Query::OpenNlSettings,
                        )],
                        vec![InlineKeyboardButton::callback(
                            "💡 Обратная связь",
                            Query::OpenFeedback,
                        )],
                        vec![InlineKeyboardButton::callback(
                            "♻️ Предложить экологическую инициативу",
                            Query::OpenFeedbackTopic {
                                topic: FeedbackTopic::SuggestEcoInitiative,
                            },
                        )],
                    ],
                }
                .into(),
            ),
        ];

        for message in messages {
            self.send_message(message).await?;
        }
        Ok(())
    }

    async fn send_kb_directory(
        &mut self,
        id: FullDirectoryId,
        item_for_move: Option<FullItemId>,
    ) -> HandlerResult<()> {
        let uctx = self.uctx();
        let message = {
            let db = &self.global_state.db;
            let mut directory = db.read_directory(uctx, id).await?;

            fn cmp<T: Ord, U>(a: &(T, U), b: &(T, U)) -> std::cmp::Ordering {
                let a_key = &a.0;
                let b_key = &b.0;
                a_key.cmp(&b_key)
            }

            directory.notes.sort_unstable_by(cmp);
            directory.directories.sort_unstable_by(cmp);

            let is_root = db.directory_parent(uctx, id).await?.is_none();
            let mut first_row = if is_root {
                Vec::with_capacity(1)
            } else {
                let mut vec = Vec::with_capacity(2);
                vec.push(InlineKeyboardButton::callback("⬆️ Вверх", Query::KbGoUp));
                vec
            };
            if item_for_move.is_none() {
                first_row.push(InlineKeyboardButton::callback(
                    "🏠 В главное меню",
                    Query::OpenMainMenu,
                ));
            } else {
                first_row.push(InlineKeyboardButton::callback(
                    "🚫 Отменить перемещение",
                    Query::GoBack,
                ));
            }

            let num_children = directory.directories.len()
                + if item_for_move.is_none() {
                    directory.notes.len()
                } else {
                    0
                };
            let mut inline_keyboard = Vec::with_capacity(2 + num_children);
            inline_keyboard.push(first_row);

            if let Some(item) = item_for_move {
                inline_keyboard.push(vec![InlineKeyboardButton::callback(
                    "↘️ Переместить сюда",
                    match item {
                        FullItemId::Note(note) => Query::KbMoveNoteHere {
                            note,
                            destination: id,
                        },
                        FullItemId::Directory(directory) => Query::KbMoveDirectoryHere {
                            directory,
                            destination: id,
                        },
                    },
                )])
            } else {
                let is_editor = self
                    .dialog
                    .data()
                    .read()
                    .unwrap()
                    .user
                    .permissions()
                    .edit_kb;
                if is_editor {
                    inline_keyboard.push(vec![InlineKeyboardButton::callback(
                        "✏️ Редактировать этот раздел",
                        Query::KbEditDir { id },
                    )])
                }
            }

            for (name, id) in directory.directories.into_iter() {
                let text = format!("📂 {}", name);
                let callback_data = Query::KbNavToDir { id };
                inline_keyboard.push(vec![InlineKeyboardButton::callback(text, callback_data)]);
            }
            if item_for_move.is_none() {
                for (name, id) in directory.notes.into_iter() {
                    let text = format!("🗒 {}", name);
                    let callback_data = Query::KbNavToNote { id };
                    inline_keyboard.push(vec![InlineKeyboardButton::callback(text, callback_data)]);
                }
            }

            let dir_description = match db.directory_name(uctx, id).await? {
                Some(name) => format!("разделе «{}»", name),
                None => String::from("корневом разделе"),
            };
            let text = match item_for_move {
                Some(FullItemId::Note(note)) => {
                    let note_name = db.note_name(uctx, note).await?;
                    if num_children == 0 {
                        STRINGS
                            .kb
                            .move_note_prompt_empty(&note_name, &dir_description)
                    } else {
                        STRINGS.kb.move_note_prompt(&note_name, &dir_description)
                    }
                }
                Some(FullItemId::Directory(dir)) => {
                    let dir_name = db
                        .directory_name(uctx, dir)
                        .await?
                        // Provide a readable and reasonable error message if we are attempting to
                        // move the root directory.
                        .ok_or(ProviderError::CannotMoveRoot)?;
                    if num_children == 0 {
                        STRINGS
                            .kb
                            .move_dir_prompt_empty(&dir_name, &dir_description)
                    } else {
                        STRINGS.kb.move_dir_prompt(&dir_name, &dir_description)
                    }
                }
                None => {
                    if num_children == 0 {
                        STRINGS.kb.dir_prompt_empty(&dir_description)
                    } else {
                        STRINGS.kb.dir_prompt(&dir_description)
                    }
                }
            };

            let reply_markup = Some(InlineKeyboardMarkup { inline_keyboard }.into());
            FormattedMessage { text, reply_markup }
        };
        self.send_message(message).await?;
        Ok(())
    }

    async fn send_kb_note(&mut self, id: FullNoteId) -> HandlerResult<()> {
        let permissions = *self.dialog.data().read().unwrap().user.permissions();
        let is_editor = permissions.edit_kb;

        let uctx = self.uctx();
        let db = &self.global_state.db;
        let note = db.read_note(uctx, id).await?;
        let note_name = db.note_name(uctx, id).await?;

        let mut inline_keyboard = Vec::with_capacity(if is_editor { 6 } else { 1 });

        if is_editor {
            inline_keyboard.push(vec![InlineKeyboardButton::callback(
                "📝 Редактировать",
                Query::KbEditNote { id },
            )]);
            inline_keyboard.push(vec![InlineKeyboardButton::callback(
                "🔤 Переименовать",
                Query::KbRenameNote { id },
            )]);
            inline_keyboard.push(vec![InlineKeyboardButton::callback(
                "➡️ Переместить в другой раздел",
                Query::KbMoveNote { id },
            )]);
            inline_keyboard.push(vec![InlineKeyboardButton::callback(
                "🗑 Удалить",
                Query::KbDeleteNote { id },
            )]);
            inline_keyboard.push(vec![InlineKeyboardButton::callback(
                "📌 Закрепить в главном меню",
                Query::KbPinNote { id },
            )]);
        }
        inline_keyboard.push(vec![
            InlineKeyboardButton::callback("⬅️ Назад", Query::GoBack),
            InlineKeyboardButton::callback("🏠 В главное меню", Query::OpenMainMenu),
        ]);

        let reply_markup = InlineKeyboardMarkup { inline_keyboard };
        let text = STRINGS.kb.note_template(&note_name).concat(note.text);
        self.send_message(FormattedMessage::with_markup(text, reply_markup.into()))
            .await?;
        Ok(())
    }

    async fn send_note_deletion_confirmation(&mut self, id: FullNoteId) -> HandlerResult<()> {
        let db = &self.global_state.db;
        let note_name = db.note_name(self.uctx(), id).await?;
        let reply_markup = InlineKeyboardMarkup {
            inline_keyboard: vec![vec![
                InlineKeyboardButton::callback("Да, удалить", Query::KbConfirmNoteDeletion { id }),
                InlineKeyboardButton::callback(
                    "Нет, не удалять",
                    Query::KbCancelNoteDeletion { id },
                ),
            ]],
        };
        // TODO: print full path.
        self.send_message(FormattedMessage::with_markup(
            STRINGS.kb.note_deletion_confirmation(&note_name),
            reply_markup.into(),
        ))
        .await?;
        Ok(())
    }

    async fn send_note_renaming_prompt(&mut self, id: FullNoteId) -> HandlerResult<()> {
        let db = &self.global_state.db;
        let note_name = db.note_name(self.uctx(), id).await?;
        let reply_markup = InlineKeyboardMarkup {
            inline_keyboard: vec![vec![InlineKeyboardButton::callback(
                "⬅️ Назад",
                Query::GoBack,
            )]],
        };
        self.send_message(FormattedMessage::with_markup(
            STRINGS.kb.note_renaming_prompt(&note_name),
            reply_markup.into(),
        ))
        .await?;
        Ok(())
    }

    async fn send_note_creation_prompt(&mut self) -> HandlerResult<()> {
        let reply_markup = InlineKeyboardMarkup {
            inline_keyboard: vec![vec![InlineKeyboardButton::callback(
                "⬅️ Назад",
                Query::GoBack,
            )]],
        };
        self.send_message(FormattedMessage::with_markup(
            STRINGS.kb.note_creation_prompt(),
            reply_markup.into(),
        ))
        .await?;
        Ok(())
    }

    async fn send_note_creation_named_prompt(&mut self) -> HandlerResult<()> {
        let reply_markup = InlineKeyboardMarkup {
            inline_keyboard: vec![vec![InlineKeyboardButton::callback(
                "⬅️ Назад",
                Query::GoBack,
            )]],
        };
        self.send_message(FormattedMessage::with_markup(
            STRINGS.kb.note_creation_named_prompt(),
            reply_markup.into(),
        ))
        .await?;
        Ok(())
    }

    async fn send_directory_editing_prompt(
        &mut self,
        destination: FullDirectoryId,
    ) -> HandlerResult<()> {
        let reply_markup = InlineKeyboardMarkup {
            inline_keyboard: vec![
                vec![InlineKeyboardButton::callback(
                    "🗒 Создать заметку",
                    Query::KbCreateNote { destination },
                )],
                vec![InlineKeyboardButton::callback(
                    "📂 Создать подраздел",
                    Query::KbCreateDirectory { destination },
                )],
                vec![InlineKeyboardButton::callback(
                    "🔤 Переименовать",
                    Query::KbRenameDirectory { id: destination },
                )],
                vec![InlineKeyboardButton::callback(
                    "➡️ Переместить в другой раздел",
                    Query::KbMoveDirectory { id: destination },
                )],
                vec![InlineKeyboardButton::callback(
                    "🗑 Удалить",
                    Query::KbDeleteDirectory { id: destination },
                )],
                // TODO: pinning.
                vec![InlineKeyboardButton::callback("⬅️ Назад", Query::GoBack)],
            ],
        };
        let name = self
            .global_state
            .db
            .directory_name(self.uctx(), destination)
            .await?;

        let text = if let Some(name) = name {
            STRINGS.kb.directory_editing_prompt(&name)
        } else {
            STRINGS.kb.root_directory_editing_prompt()
        };
        self.send_message(FormattedMessage::with_markup(text, reply_markup.into()))
            .await?;
        Ok(())
    }

    async fn send_note_editing_prompt(&mut self, note: FullNoteId) -> HandlerResult<()> {
        let reply_markup = InlineKeyboardMarkup {
            inline_keyboard: vec![vec![InlineKeyboardButton::callback(
                "⬅️ Назад",
                Query::GoBack,
            )]],
        };
        let name = self.global_state.db.note_name(self.uctx(), note).await?;
        self.send_message(FormattedMessage::with_markup(
            STRINGS.kb.note_editing_prompt(&name),
            reply_markup.into(),
        ))
        .await?;
        Ok(())
    }

    async fn send_directory_creation_prompt(&mut self) -> HandlerResult<()> {
        let reply_markup = InlineKeyboardMarkup {
            inline_keyboard: vec![vec![InlineKeyboardButton::callback(
                "⬅️ Назад",
                Query::GoBack,
            )]],
        };
        self.send_message(FormattedMessage::with_markup(
            STRINGS.kb.directory_creation_prompt(),
            reply_markup.into(),
        ))
        .await?;
        Ok(())
    }

    async fn send_directory_renaming_prompt(&mut self, id: FullDirectoryId) -> HandlerResult<()> {
        let reply_markup = InlineKeyboardMarkup {
            inline_keyboard: vec![vec![InlineKeyboardButton::callback(
                "⬅️ Назад",
                Query::GoBack,
            )]],
        };
        let old_name = self
            .global_state
            .db
            .directory_name(self.uctx(), id)
            .await?
            .ok_or(ProviderError::CannotRenameRoot)?;
        self.send_message(FormattedMessage::with_markup(
            STRINGS.kb.directory_renaming_prompt(&old_name),
            reply_markup.into(),
        ))
        .await?;
        Ok(())
    }

    async fn send_directory_deletion_confirmation(
        &mut self,
        id: FullDirectoryId,
    ) -> HandlerResult<()> {
        let db = &self.global_state.db;
        let directory_name = db
            .directory_name(self.uctx(), id)
            .await?
            .ok_or(ProviderError::CannotDeleteRoot)?;
        let reply_markup = InlineKeyboardMarkup {
            inline_keyboard: vec![vec![
                InlineKeyboardButton::callback(
                    "Да, удалить",
                    Query::KbConfirmDirectoryDeletion { id },
                ),
                InlineKeyboardButton::callback(
                    "Нет, не удалять",
                    Query::KbCancelDirectoryDeletion { id },
                ),
            ]],
        };
        // TODO: print full path.
        self.send_message(FormattedMessage::with_markup(
            STRINGS.kb.directory_deletion_confirmation(&directory_name),
            reply_markup.into(),
        ))
        .await?;
        Ok(())
    }

    async fn send_feedback_prompt(&mut self) -> HandlerResult<()> {
        let reply_markup = InlineKeyboardMarkup {
            inline_keyboard: vec![
                vec![InlineKeyboardButton::callback(
                    "Зелёная Вышка",
                    Query::OpenFeedbackTopic {
                        topic: FeedbackTopic::HseGreen,
                    },
                )],
                vec![InlineKeyboardButton::callback(
                    "Чат-бот",
                    Query::OpenFeedbackTopic {
                        topic: FeedbackTopic::Bot,
                    },
                )],
                vec![InlineKeyboardButton::callback(
                    "Предложить экологическую инициативу",
                    Query::OpenFeedbackTopic {
                        topic: FeedbackTopic::SuggestEcoInitiative,
                    },
                )],
                vec![InlineKeyboardButton::callback(
                    "Сообщить о несанкционированной свалке",
                    Query::OpenFeedbackTopic {
                        topic: FeedbackTopic::ReportGarbageDump,
                    },
                )],
                vec![InlineKeyboardButton::callback(
                    "Другое",
                    Query::OpenFeedbackTopic {
                        topic: FeedbackTopic::Other,
                    },
                )],
                vec![InlineKeyboardButton::callback(
                    "В главное меню",
                    Query::OpenMainMenu,
                )],
            ],
        };
        self.send_message(FormattedMessage::with_markup(
            STRINGS.feedback.prompt(),
            reply_markup.into(),
        ))
        .await?;
        Ok(())
    }

    async fn send_form_filling_prompt(&mut self, fil: states::FormFilling) -> HandlerResult<()> {
        let elem = fil.form_state.current_element();
        let text = &elem.text;
        let markup = match &elem.input_type {
            FormInputType::Choice { options } => {
                let inline_keyboard = options
                    .iter()
                    .enumerate()
                    .map(|(i, opt)| {
                        vec![InlineKeyboardButton::callback(
                            opt.clone(),
                            Query::FormOption { index: i },
                        )]
                    })
                    .collect();
                Some(InlineKeyboardMarkup { inline_keyboard }.into())
            }
            FormInputType::Location => Some(
                KeyboardMarkup {
                    keyboard: vec![vec![KeyboardButton::new(String::from(
                        "Отправить местоположение",
                    ))
                    .request(ButtonRequest::Location)]],
                    one_time_keyboard: Some(true),
                    resize_keyboard: Some(true),
                    input_field_placeholder: Some(String::from("Или введите адрес")),
                    selective: None,
                }
                .into(),
            ),
            _ => None,
        };
        let message = FormattedMessage {
            text: FormattedText {
                raw_text: text.clone(),
                entities: None,
            },
            reply_markup: markup,
        };
        self.send_message(message).await?;
        Ok(())
    }

    fn start_feedback_form_filling(&mut self, topic: FeedbackTopic) {
        let identity_element = FormElement {
            text: String::from("Введите Ваши ФИО:"),
            input_type: FormInputType::ShortText,
        };
        let form = match topic {
            FeedbackTopic::ReportGarbageDump => Form {
                elements: vec![
                    identity_element,
                    FormElement {
                        text: String::from("Местоположение или адрес свалки:"),
                        input_type: FormInputType::Location,
                    },
                    FormElement {
                        text: String::from("Контактный телефон:"),
                        input_type: FormInputType::ShortText,
                    },
                    FormElement {
                        text: String::from("Контактный email при наличии:"),
                        input_type: FormInputType::ShortText,
                    },
                    FormElement {
                        text: String::from("Опишите подробности, которые могут быть важными:"),
                        input_type: FormInputType::Message,
                    },
                ],
            },
            _ => Form {
                elements: vec![
                    identity_element,
                    FormElement {
                        text: String::from("Введите ваше сообщение:"),
                        input_type: FormInputType::Message,
                    },
                ],
            },
        };

        let form_state = FormFillingState::new(form);
        let current_state = self.state();
        let state = states::FormFilling {
            form_state,
            completion_state: Box::new(DialogState::MainMenu),
            return_state: Box::new(current_state),
            on_completion: self
                .global_state
                .feedback_tx
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .clone(),
        };
        self.set_state(DialogState::FormFilling(state));
    }

    async fn send_subscriptions_menu(&mut self) -> HandlerResult<()> {
        let nl = &STRINGS.newsletter;
        let subscriptions = self
            .dialog
            .data()
            .read()
            .unwrap()
            .user
            .subscriptions()
            .clone();

        let mut text = nl.menu_head();
        for &(ref name, ref description, ref is_allowed) in self.global_state.newsletters.iter() {
            if !is_allowed(self.dialog.data().read().unwrap().user.permissions()) {
                continue;
            }

            let item = if subscriptions.contains(name) {
                nl.menu_item_subscribed(description)
            } else {
                nl.menu_item_not_subscribed(description)
            };
            text = text.concat(item);
        }

        let newsletter_buttons_iter = self.global_state.newsletters.iter().flat_map(
            |&(ref name, ref desc, ref is_allowed)| {
                if !is_allowed(self.dialog.data().read().unwrap().user.permissions()) {
                    return None;
                }
                let subscribed = subscriptions.contains(name);
                let action_text = if subscribed {
                    "Отписаться"
                } else {
                    "Подписаться"
                };
                Some(vec![InlineKeyboardButton::callback(
                    format!("{} — {}", desc, &action_text),
                    if subscribed {
                        Query::Unsubscribe {
                            newsletter: name.clone(),
                        }
                    } else {
                        Query::Subscribe {
                            newsletter: name.clone(),
                        }
                    },
                )])
            },
        );

        let buttons_iter = std::iter::once(vec![InlineKeyboardButton::callback(
            "⬅️ Назад",
            Query::GoBack,
        )])
        .chain(newsletter_buttons_iter);

        let markup = InlineKeyboardMarkup {
            inline_keyboard: buttons_iter.collect(),
        };

        self.send_message(FormattedMessage::with_markup(text, markup.into()))
            .await?;
        Ok(())
    }
}
