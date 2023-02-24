use crate::db::AccessTask;
use crate::dispatch::DialogStorage;
use crate::global_state::GlobalState;
use crate::kb::Tree;
use crate::message_queue::MessageQueue;
use crate::newsletter::{feedback::FeedbackNewsletter};
use crate::newsletter::{Newsletter, NewsletterWorker};
use crate::types::BotType;
use crate::ui;
use crate::util::UnsafeRc;
use std::error::Error;
use std::sync::{Arc, Mutex};
use teloxide::adaptors::throttle::Limits;
use teloxide::prelude::*;

/// The application with its state.
pub struct App {
    bot: BotType,
}

impl App {
    /// Create an application. The Telegram Bot API token must be provided.
    pub fn new(api_token: String) -> Self {
        Self {
            bot: Bot::new(api_token).auto_send(), // .throttle(Limits::default()).auto_send(),
        }
    }

    /// Run the application.
    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        let message_handler = Update::filter_message().endpoint(ui::handle_message);

        let callback_query_handler =
            Update::filter_callback_query().endpoint(ui::handle_callback_query);

        let root_handler = teloxide::dptree::entry()
            .branch(message_handler)
            .branch(callback_query_handler);

        let db = rusqlite::Connection::open("hse-eco-bot.sqlite")?;
        db.execute("PRAGMA foreign_keys=ON", rusqlite::params![])?;
        let dialog_storage = DialogStorage::new(&db);

        let (feedback_newsletter, feedback_tx) = FeedbackNewsletter::new();

        let newsletters: &[&dyn Newsletter] = &[&feedback_newsletter];

        // SAFETY: clones of [`db_rc`] are never shared between threads.
        let db_rc = unsafe { UnsafeRc::new(db) };
        let (kb_tree, _provider_registry, newsletter_sink) =
            unsafe { Tree::new(db_rc, newsletters) };
        let (db_access_task, db_cmd_sender) = AccessTask::new(kb_tree, newsletter_sink);
        let db_access_task_handle = db_access_task.spawn();
        let global_state = Arc::new(GlobalState {
            dialog_storage,
            db: db_cmd_sender,
            feedback_tx: Mutex::new(None),
            newsletters: newsletters
                .iter()
                .copied()
                .map(|nl| (nl.name(), nl.description(), nl.allowed()))
                .collect(),
        });

        let (message_queue, message_queue_tx) = MessageQueue::new();
        tokio::spawn(message_queue.run(self.bot.clone()));

        tokio::spawn(
            NewsletterWorker::new(
                self.bot.clone(),
                feedback_newsletter,
                Arc::clone(&global_state),
                message_queue_tx.clone(),
            )
            .manage(),
        );
        *global_state.feedback_tx.lock().unwrap() = Some(feedback_tx);

        let mut dispatcher = Dispatcher::builder(self.bot, root_handler)
            .dependencies(teloxide::dptree::deps![global_state, message_queue_tx])
            .build();
        dispatcher.dispatch().await;
        db_access_task_handle.abort();
        let _ = db_access_task_handle.await;

        Ok(())
    }
}

#[allow(dead_code, unreachable_code)]
fn assert_traits() {
    panic!("This function must not be called");

    fn declval<T>() -> T {
        panic!("declval")
    }

    fn must_be_send(_: impl Send) {}

    must_be_send(ui::handle_message(
        declval(),
        declval(),
        declval(),
        declval(),
    ));
    must_be_send(ui::handle_callback_query(
        declval(),
        declval(),
        declval(),
        declval(),
    ));
}
