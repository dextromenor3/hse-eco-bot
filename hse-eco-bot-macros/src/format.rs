use proc_macro2::TokenStream;
use quote::{format_ident, quote};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Item {
    Literal(String),
    Entity(Entity),
    Placeholder(String),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Entity {
    pub kind: String,
    pub params: Vec<String>,
    pub inner: Vec<Item>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ParseError {
    ExpectedEnd,
    MissingClosingBrace,
    MissingClosingParen,
    NothingToEscape,
    EntityKindIsEmpty,
    UnfinishedEntity,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Parsed {
    pub items: Vec<Item>,
}

pub struct Parser<'a> {
    iter: std::iter::Peekable<std::str::CharIndices<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(string: &'a str) -> Self {
        Self {
            iter: string.char_indices().peekable(),
        }
    }

    pub fn parse(&mut self) -> Result<Parsed, ParseError> {
        let items = self.parse_string()?;

        if let Some(_) = self.iter.peek() {
            Err(ParseError::ExpectedEnd)
        } else {
            Ok(Parsed { items })
        }
    }

    fn parse_string(&mut self) -> Result<Vec<Item>, ParseError> {
        let mut items = Vec::new();
        loop {
            let peek = self.iter.peek().map(|x| *x);
            match peek {
                Some((_, '{')) => items.push(Item::Placeholder(self.parse_placeholder()?)),
                Some((_, '@')) => items.push(Item::Entity(self.parse_entity()?)),
                Some((_, ')')) | None => return Ok(items),
                Some(_) => items.push(Item::Literal(self.parse_literal()?)),
            }
        }
    }

    fn parse_placeholder(&mut self) -> Result<String, ParseError> {
        let (_, opening_brace) = self.iter.next().unwrap();
        assert_eq!(opening_brace, '{');
        let mut string = String::new();
        loop {
            match self.iter.next() {
                Some((_, '}')) => return Ok(string),
                Some((_, c)) => string.push(c),
                None => return Err(ParseError::MissingClosingBrace),
            }
        }
    }

    fn parse_entity(&mut self) -> Result<Entity, ParseError> {
        assert_eq!(self.iter.next().unwrap().1, '@');
        let kind = self.parse_entity_kind()?;
        let params = self.parse_entity_params()?;
        let inner = self.parse_entity_inner()?;
        Ok(Entity {
            kind,
            params,
            inner,
        })
    }

    fn parse_entity_kind(&mut self) -> Result<String, ParseError> {
        let mut kind = String::new();
        loop {
            match self.iter.peek() {
                Some(&(_, c)) if c.is_ascii_alphabetic() => {
                    let _ = self.iter.next();
                    kind.push(c);
                }
                Some(&(_, _)) => {
                    if kind.is_empty() {
                        return Err(ParseError::EntityKindIsEmpty);
                    } else {
                        return Ok(kind);
                    }
                }
                None => return Err(ParseError::UnfinishedEntity),
            }
        }
    }

    fn parse_entity_params(&mut self) -> Result<Vec<String>, ParseError> {
        // TODO: stub.
        Ok(Vec::new())
    }

    fn parse_entity_inner(&mut self) -> Result<Vec<Item>, ParseError> {
        match self.iter.next() {
            Some((_, '(')) => (),
            _ => return Err(ParseError::UnfinishedEntity),
        }
        let items = self.parse_string()?;
        match self.iter.next() {
            Some((_, ')')) => (),
            _ => return Err(ParseError::MissingClosingParen),
        }
        Ok(items)
    }

    fn parse_literal(&mut self) -> Result<String, ParseError> {
        let mut string = String::new();
        loop {
            match self.iter.peek() {
                Some(&(_, ')' | '@' | '{')) | None => return Ok(string),
                Some(&(_, '\\')) => {
                    let _ = self.iter.next();
                    if let Some((_, c)) = self.iter.next() {
                        string.push(c);
                    } else {
                        return Err(ParseError::NothingToEscape);
                    }
                }
                Some(&(_, c)) => {
                    let _ = self.iter.next();
                    string.push(c);
                }
            }
        }
    }
}

impl Parsed {
    pub fn generate_code(&self) -> (TokenStream, usize) {
        let mut param_counter = 0;
        let stream = Self::process_items(&self.items, &mut param_counter);
        let code = quote! {
            let mut raw_text = String::new();
            #[allow(dead_code)]
            let mut utf16_count = 0_usize;
            let mut entities = Vec::new();

            #stream

            crate::message::FormattedText {
                raw_text,
                entities: Some(entities),
            }
        };
        (code.into(), param_counter)
    }

    fn process_items(items: &[Item], param_counter: &mut usize) -> TokenStream {
        let mut stream = TokenStream::new();
        for item in items {
            stream.extend(Self::process_item(item, param_counter));
        }
        stream
    }

    fn process_item(item: &Item, param_counter: &mut usize) -> TokenStream {
        match item {
            Item::Literal(s) => {
                quote! {
                    {
                        let string = #s;
                        raw_text.push_str(string);
                        utf16_count += string.encode_utf16().count();
                    }
                }
            }
            Item::Entity(e) => {
                let inner_tokens = Self::process_items(&e.inner, param_counter);
                let entity_kind = match e.kind.as_str() {
                    "bold" => quote! { Bold },
                    "italic" => quote! { Italic },
                    "code" => quote! { Code },
                    "pre" => quote! { Pre { language: None } },
                    _ => panic!("Unsupported entity kind: {:?}", &e.kind),
                };
                quote! {
                    {
                        let old_utf16_count = utf16_count;
                        #inner_tokens
                        entities.push(::teloxide::types::MessageEntity {
                            kind: ::teloxide::types::MessageEntityKind::#entity_kind,
                            offset: old_utf16_count,
                            length: utf16_count - old_utf16_count,
                        });
                    }
                }
            }
            Item::Placeholder(spec) => {
                let full_spec = format!("{{{}}}", spec);
                let param_name = format_ident!("param_{}", *param_counter + 1);
                *param_counter += 1;
                quote! {
                    {
                        use std::fmt::Write;
                        let old_byte_size = raw_text.as_bytes().len();
                        write!(raw_text, #full_spec, #param_name).unwrap();
                        utf16_count += raw_text[old_byte_size..].encode_utf16().count();
                    }
                }
            }
        }
        .into()
    }
}
