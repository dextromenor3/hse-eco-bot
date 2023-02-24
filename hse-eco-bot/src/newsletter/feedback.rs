use super::{Newsletter, NewsletterMessage, NoFilter};
use crate::message::FormattedText;
use crate::ui::form::{Form, FormInput};
use std::future::Future;
use std::pin::Pin;
use teloxide::types::MessageEntity;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;
use crate::user::Permissions;

pub struct FeedbackNewsletter {
    form_response_rx: Mutex<Receiver<(Form, Vec<FormInput>)>>,
}

impl FeedbackNewsletter {
    pub fn new() -> (Self, Sender<(Form, Vec<FormInput>)>) {
        let (form_response_tx, form_response_rx) = mpsc::channel(16);
        let form_response_rx = Mutex::new(form_response_rx);
        (Self { form_response_rx }, form_response_tx)
    }
}

impl Newsletter for FeedbackNewsletter {
    fn name(&self) -> String {
        String::from("feedback")
    }

    fn description(&self) -> String {
        String::from("Обратная связь")
    }

    fn allowed(&self) -> Box<dyn Fn(&Permissions) -> bool + Send + Sync> {
        Box::new(|p| p.receive_feedback)
    }

    fn tags(&self) -> String {
        String::from("#обратнаясвязь")
    }

    fn wait_until_ready(&self) -> Pin<Box<dyn Future<Output = NewsletterMessage> + Send + '_>> {
        Box::pin(async {
            let (form, input) = self.form_response_rx.lock().await.recv().await.unwrap();
            let text = form
                .elements
                .into_iter()
                .zip(input.into_iter())
                .map(|(elem, input)| {
                    let elem_entities =
                        vec![MessageEntity::bold(0, elem.text.encode_utf16().count())];
                    let elem_fmt = FormattedText {
                        raw_text: format!("{}\n", elem.text),
                        entities: Some(elem_entities),
                    };
                    // TODO: media.
                    let input_fmt = match input {
                        FormInput::ShortText { text } => FormattedText {
                            raw_text: text,
                            entities: None,
                        },
                        FormInput::Text { text } => text,
                        FormInput::Number { number } => FormattedText {
                            raw_text: number.to_string(),
                            entities: None,
                        },
                        FormInput::Location { location } => FormattedText {
                            raw_text: location.to_string(),
                            entities: None,
                        },
                        _ => FormattedText { raw_text: String::from("<unimplemented>"), entities: None },
                    };
                    elem_fmt.concat(input_fmt)
                })
                .fold(None, |cat: Option<FormattedText>, new| match cat {
                    Some(x) => Some(
                        x.concat(FormattedText {
                            raw_text: String::from("\n\n"),
                            entities: None,
                        })
                        .concat(new),
                    ),
                    None => Some(new),
                })
                .unwrap();

            NewsletterMessage {
                text,
                tags: None,
                user_filter: Box::new(NoFilter),
            }
        })
    }
}
