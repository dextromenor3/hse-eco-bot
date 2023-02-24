use teloxide::types::{MessageEntity, ReplyMarkup};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FormattedText {
    pub raw_text: String,
    pub entities: Option<Vec<MessageEntity>>,
}

impl FormattedText {
    pub fn concat(mut self, other: Self) -> Self {
        let entity_offset = self.raw_text.encode_utf16().count();
        self.raw_text.push_str(&other.raw_text);
        match (&mut self.entities, other.entities) {
            (Some(ref mut left), Some(right)) => {
                left.extend(right.into_iter().map(|mut ent| {
                    ent.offset += entity_offset;
                    ent
                }));
            }
            (None, Some(mut right)) => {
                for ent in right.iter_mut() {
                    ent.offset += entity_offset;
                }
                self.entities = Some(right)
            }
            (_, None) => (),
        }
        self
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FormattedMessage {
    pub text: FormattedText,
    pub reply_markup: Option<ReplyMarkup>,
}

impl FormattedMessage {
    pub fn new(text: FormattedText) -> Self {
        Self {
            text,
            reply_markup: None,
        }
    }

    pub fn with_markup(text: FormattedText, reply_markup: ReplyMarkup) -> Self {
        Self {
            text,
            reply_markup: Some(reply_markup),
        }
    }
}

impl From<FormattedText> for FormattedMessage {
    fn from(text: FormattedText) -> Self {
        Self::new(text)
    }
}
