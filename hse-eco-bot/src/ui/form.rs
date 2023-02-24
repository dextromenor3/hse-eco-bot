use crate::media::{Image, Location, LocationOrAddress};
use crate::message::{FormattedMessage, FormattedText};
use crate::strings::STRINGS;
use crate::user_facing_error::UserFacingError;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Form {
    pub elements: Vec<FormElement>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FormElement {
    pub text: String,
    pub input_type: FormInputType,
}

impl FormElement {
    fn parse_input(&self, input: FormRawInput) -> Result<FormInput, FormInputError> {
        self.input_type
            .parse_input(input)
            .map_err(|input| FormInputError {
                element: self.clone(),
                input,
            })
    }
}
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FormInputType {
    Choice { options: Vec<String> },
    Number,
    ShortText,
    Text,
    Message,
    Image,
    ImageGallery,
    Location,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormInput {
    Choice { index: usize },
    Number { number: u64 },
    ShortText { text: String },
    Text { text: FormattedText },
    //Message { TODO },
    Image { image: Image },
    ImageGallery { images: Vec<Image> },
    Location { location: LocationOrAddress },
}

impl FormInputType {
    fn parse_input(&self, input: FormRawInput) -> Result<FormInput, FormRawInput> {
        match &self {
            Self::Choice { ref options } => match input {
                FormRawInput::Choice { index } if index < options.len() => {
                    Ok(FormInput::Choice { index })
                }
                _ => Err(input),
            },
            Self::Number => match input {
                FormRawInput::Text { ref text } => match text.parse() {
                    Ok(number) => Ok(FormInput::Number { number }),
                    Err(_) => Err(input),
                },
                _ => Err(input),
            },
            Self::ShortText => match input {
                FormRawInput::Text { text } if text.encode_utf16().count() <= 100 => {
                    Ok(FormInput::Text {
                        text: FormattedText {
                            raw_text: text,
                            entities: None,
                        },
                    })
                }
                _ => Err(input),
            },
            Self::Text => match input {
                FormRawInput::Text { text } if text.encode_utf16().count() <= 3500 => {
                    Ok(FormInput::Text {
                        text: FormattedText {
                            raw_text: text,
                            entities: None,
                        },
                    })
                }
                FormRawInput::FormattedText { text }
                    if text.raw_text.encode_utf16().count() <= 3500 =>
                {
                    Ok(FormInput::Text { text })
                }
                _ => Err(input),
            },
            Self::Message => match input {
                FormRawInput::Text { text } if text.encode_utf16().count() <= 3500 => {
                    Ok(FormInput::Text {
                        text: FormattedText {
                            raw_text: text,
                            entities: None,
                        },
                    })
                }
                FormRawInput::FormattedText { text }
                    if text.raw_text.encode_utf16().count() <= 3500 =>
                {
                    Ok(FormInput::Text { text })
                }
                FormRawInput::Message { message } => {
                    // TODO: attachments.
                    Ok(FormInput::Text { text: message.text })
                }
                _ => Err(input),
            },
            // TODO.
            Self::Image => Err(input),
            // TODO.
            Self::ImageGallery => Err(input),
            Self::Location => match input {
                FormRawInput::Text { text } => Ok(FormInput::Location {
                    location: LocationOrAddress::Address(text),
                }),
                FormRawInput::Location { location } => Ok(FormInput::Location {
                    location: LocationOrAddress::Location(location),
                }),
                _ => Err(input),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormRawInput {
    Choice { index: usize },
    Text { text: String },
    FormattedText { text: FormattedText },
    Location { location: Location },
    Message { message: FormattedMessage },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormInputError {
    pub element: FormElement,
    pub input: FormRawInput,
}

impl Display for FormInputError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Invalid input {:?} for form element {:?}",
            &self.input, &self.element,
        )
    }
}

impl Error for FormInputError {}

impl UserFacingError for FormInputError {
    fn user_message(&self) -> FormattedText {
        STRINGS.form.invalid_input()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormFillingState {
    form: Form,
    input: Vec<FormInput>,
}

impl FormFillingState {
    pub fn new(form: Form) -> Self {
        let num_elements = form.elements.len();
        Self {
            form,
            input: Vec::with_capacity(num_elements),
        }
    }

    pub fn back(&mut self) {
        if self.can_go_back() {
            self.input.pop().unwrap();
        } else {
            panic!(concat!(
                "Cannot go back in the form filling process, ",
                "since no input from the user has been seen yet",
            ));
        }
    }

    pub fn can_go_back(&self) -> bool {
        !self.input.is_empty()
    }

    pub fn next(&mut self, input: FormRawInput) -> Result<(), FormInputError> {
        if self.is_done() {
            panic!("Cannot proceed with the form filling process, since it has alredy finished");
        }

        let index = self.input.len();
        let element = &self.form.elements[index];
        self.input.push(element.parse_input(input)?);
        Ok(())
    }

    pub fn is_done(&self) -> bool {
        self.input.len() == self.form.elements.len()
    }

    pub fn current_element(&self) -> &FormElement {
        if self.is_done() {
            panic!("Completed form has no current element");
        }

        &self.form.elements[self.input.len()]
    }

    pub fn into_parts(self) -> (Form, Vec<FormInput>) {
        (self.form, self.input)
    }
}
