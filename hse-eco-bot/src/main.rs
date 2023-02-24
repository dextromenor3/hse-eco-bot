#[macro_use]
extern crate log;

#[cfg(test)]
mod tests;

mod app;
mod callback_query;
mod db;
mod dispatch;
mod feedback;
mod global_state;
mod invalid_action;
mod kb;
mod media;
mod message;
mod message_format_error;
mod message_queue;
mod newsletter;
mod state;
mod strings;
mod types;
mod ui;
mod user;
mod user_error;
mod user_facing_error;
mod util;

use crate::app::App;
use std::env;
use std::error::Error;

/// A wrapper around [`std::env::VarError`] containing the variable name that has caused the error.
#[derive(Debug, Clone)]
struct EnvError {
    inner: std::env::VarError,
    variable_name: String,
}

impl std::fmt::Display for EnvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cannot read environment variable `{}`: {}",
            self.variable_name, self.inner
        )
    }
}

impl Error for EnvError {}

/// Read the bot API token from an environment variable.
fn read_api_token() -> Result<String, EnvError> {
    const VAR_NAME: &'static str = "C";
    // Wrap error to include the variable name.
    env::var(VAR_NAME).map_err(|e| EnvError {
        inner: e,
        variable_name: String::from(VAR_NAME),
    })
}

async fn fallible_main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    let app = App::new(read_api_token()?);
    app.run().await
}

#[tokio::main]
async fn main() {
    // Handle errors in a custom way manually. Returning a `Result` would
    // not allow for such degree of customatization of output.
    match fallible_main().await {
        Ok(()) => (),
        Err(e) => {
            error!("The bot has terminated because of an error: {}", e);

            let mut current_error = e.as_ref();
            while let Some(cause) = current_error.source() {
                error!("This error has been caused by another error: {}", cause);
                current_error = cause;
            }

            std::process::exit(1)
        }
    }
}
