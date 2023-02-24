use crate::dispatch::UserDialog;
use crate::message::FormattedMessage;
use crate::types::{BotType, HandlerError, HandlerResult, InternalError};
use teloxide::errors::RequestError;
use teloxide::types::ChatId;
use tokio::sync::{mpsc, oneshot};

pub struct MessagePackage {
    pub message: FormattedMessage,
    pub chat_id: ChatId,
    pub result_tx: oneshot::Sender<HandlerResult<()>>,
}

impl std::fmt::Debug for MessagePackage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessagePackage")
            .field("message", &self.message)
            .field("chat_id", &self.chat_id)
            .finish_non_exhaustive()
    }
}

pub struct MessageQueue {
    rx: mpsc::Receiver<MessagePackage>,
}

#[derive(Clone)]
pub struct MessageQueueSender {
    tx: mpsc::Sender<MessagePackage>,
}

impl MessageQueueSender {
    pub async fn send_message(
        &mut self,
        message: FormattedMessage,
        chat_id: ChatId,
    ) -> HandlerResult<()> {
        let (result_tx, result_rx) = oneshot::channel();
        let pkg = MessagePackage {
            message,
            chat_id,
            result_tx,
        };
        self.tx.send(pkg).await.unwrap();
        result_rx.await.unwrap()
    }
}

impl MessageQueue {
    pub fn new() -> (Self, MessageQueueSender) {
        let (tx, rx) = mpsc::channel(1);
        (Self { rx }, MessageQueueSender { tx })
    }

    pub async fn run(mut self, bot: BotType) -> HandlerResult<()> {
        while let Some(pkg) = self.rx.recv().await {
            match UserDialog::send_message_with_id(pkg.chat_id, &bot, pkg.message.clone()).await {
                Err(HandlerError::Internal(InternalError::Teloxide(RequestError::RetryAfter(
                    duration,
                )))) => tokio::time::sleep(duration).await,
                x => {
                    pkg.result_tx.send(x).unwrap();
                }
            }
        }
        debug!("Message queue closed");
        Ok(())
    }
}
