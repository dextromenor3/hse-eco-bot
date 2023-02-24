use crate::db::CommandSender;
use crate::dispatch::DialogStorage;
use crate::ui::form::{Form, FormInput};
use std::sync::Mutex;
use tokio::sync::mpsc::Sender;
use crate::user::Permissions;

pub struct GlobalState {
    pub dialog_storage: DialogStorage,
    pub db: CommandSender,
    pub feedback_tx: Mutex<Option<Sender<(Form, Vec<FormInput>)>>>,
    pub newsletters: Vec<(String, String, Box<dyn Fn(&Permissions) -> bool + Send + Sync>)>,
}
