pub mod archive;
pub mod feedback;

use crate::dispatch::UserDialog;
use crate::global_state::GlobalState;
use crate::message_queue::MessageQueueSender;
use crate::kb::command::Command;
use crate::kb::{Note, ProviderId};
use crate::message::{FormattedMessage, FormattedText};
use crate::state::DialogState;
use crate::strings::STRINGS;
use crate::types::{BotType, HandlerResult};
use crate::ui::form::{Form, FormInput};
use crate::user::Permissions;
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use teloxide::types::UserId;
use tokio::sync::mpsc::Sender;

pub trait UserFilter {
    fn should_skip_user(&self, user_id: UserId) -> bool;
}

pub struct NoFilter;

impl UserFilter for NoFilter {
    fn should_skip_user(&self, _user_id: UserId) -> bool {
        false
    }
}

pub struct NewsletterMessage {
    pub text: FormattedText,
    pub user_filter: Box<dyn UserFilter + Send>,
    pub tags: Option<String>,
}

pub trait Newsletter {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn allowed(&self) -> Box<dyn Fn(&Permissions) -> bool + Send + Sync>;
    fn tags(&self) -> String;
    fn wait_until_ready(&self) -> Pin<Box<dyn Future<Output = NewsletterMessage> + Send + '_>>;
}

pub struct NewsletterWorker<N> {
    bot: BotType,
    newsletter: N,
    global_state: Arc<GlobalState>,
    message_queue_tx: MessageQueueSender,
}

impl<N> NewsletterWorker<N>
where
    N: Newsletter + Send,
{
    pub fn new(bot: BotType, newsletter: N, global_state: Arc<GlobalState>, message_queue_tx: MessageQueueSender) -> Self {
        Self {
            bot,
            newsletter,
            global_state,
            message_queue_tx,
        }
    }

    pub async fn manage(mut self) -> HandlerResult<()> {
        let name = self.newsletter.name();

        loop {
            let nl_message = self.newsletter.wait_until_ready().await;
            let all_tags = match nl_message.tags {
                Some(s) => format!("{} {}", self.newsletter.tags(), s),
                None => self.newsletter.tags(),
            };

            let message =
                FormattedMessage::new(STRINGS.newsletter.header(&all_tags).concat(nl_message.text));
            let mut dialogs = Vec::new();
            self.global_state
                .dialog_storage
                .inspect_dialogs(&mut |_user_id, dialog| dialogs.push(Arc::clone(dialog)));
            debug!("Sending newsletter `{}`", &name);
            let text = message.text.clone();
            let name_clone = name.clone();
            self.global_state
                .db
                .send(Command::new(move |ctx| {
                    // TODO: save media.
                    ctx.newsletter_sink
                        .store(&name_clone, Note { text }, chrono::Local::now())
                }))
                .await?;
            for dialog in dialogs {
                let (should_send, state) = {
                    let dialog_data = dialog.data().read().unwrap();
                    let is_subscribed = dialog_data.user.subscriptions().contains(&name);
                    let is_allowed = self.newsletter.allowed()(dialog_data.user.permissions());
                    let state = dialog_data.state.clone();
                    (is_subscribed && is_allowed, state)
                };
                if !should_send {
                    continue;
                }

                match state {
                    DialogState::Initial => (),
                    DialogState::MainMenu => {
                        match self.message_queue_tx.send_message(message.clone(), dialog.chat_id()).await {
                            Ok(_) => (),
                            Err(e) => warn!("Error sending newsletter message: {}", &e),
                        }
                    }
                    _ => {
                        tokio::task::spawn(worker_retry_loop(
                            self.bot.clone(),
                            message.clone(),
                            dialog,
                            self.message_queue_tx.clone(),
                        ));
                    }
                }
            }
        }
    }
}

async fn worker_retry_loop(bot: BotType, message: FormattedMessage, dialog: Arc<UserDialog>, mut message_queue_tx: MessageQueueSender) {
    let starting_time = Instant::now();

    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let now = Instant::now();
        if (now - starting_time).as_secs() >= 30 {
            trace!("worker_retry_loop: giving up");
            break;
        }

        trace!("worker_retry_loop: retrying");
        let state = dialog.data().read().unwrap().state.clone();
        match state {
            DialogState::Initial => break,
            DialogState::MainMenu => {
                if let Err(e) = message_queue_tx.send_message(message, dialog.chat_id()).await {
                    trace!("worker_retry_loop: send error: {}", &e);
                }
                break;
            }
            _ => (),
        }
    }
}
