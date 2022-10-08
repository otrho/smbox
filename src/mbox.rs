use std::{collections::HashMap, io, iter::once};

use itertools::Itertools;

// -------------------------------------------------------------------------------------------------

pub fn get_mbox_path() -> io::Result<String> {
    std::env::vars_os()
        .find(|(key, _)| key == "MAIL")
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Unable to determine mbox path; missing MAIL environment variable.",
            )
        })
        .and_then(|(_, env_value)| {
            env_value.into_string().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Malformed string in MAIL environment variable.",
                )
            })
        })
}

// -------------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum FieldType {
    Date,
    From,
    Subject,
    Status,
    Body, // Not a field, but... kinda.
}

#[derive(Debug)]
struct Message {
    start_idx: usize,
    end_idx: usize, // Index to line *after* last line in message.
    field_idcs: HashMap<FieldType, usize>,
}

impl Message {
    fn new(start_idx: usize, end_idx: usize, field_idcs: HashMap<FieldType, usize>) -> Self {
        Message {
            start_idx,
            end_idx,
            field_idcs,
        }
    }
}

// -------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct Mbox {
    lines: Vec<String>,
    messages: Vec<Message>,
}

impl Mbox {
    pub fn from_lines(mbox_lines: Vec<String>) -> Self {
        let field_parser = FieldsParser::default();
        let messages = mbox_lines
            .iter()
            .enumerate()
            .filter_map(|(idx, line)| line.starts_with("From ").then_some(idx))
            .chain(once(mbox_lines.len()))
            .tuple_windows()
            .map(|(start_idx, end_idx)| {
                Message::new(
                    start_idx,
                    end_idx,
                    field_parser.gather_fields(&mbox_lines[start_idx..end_idx]),
                )
            })
            .collect::<Vec<_>>();

        Mbox {
            lines: mbox_lines,
            messages,
        }
    }

    pub(crate) fn count(&self) -> usize {
        self.messages.len()
    }

    pub(crate) fn msg_at(&self, idx: usize) -> Option<MessageCursor> {
        (idx < self.messages.len()).then_some(MessageCursor::new(&self.messages[idx], &self.lines))
    }

    pub(crate) fn iter(&self) -> Iter {
        Iter::new(self)
    }

//    pub(crate) fn iter_mut(&mut self) -> IterMut {
//        IterMut::new(self)
//    }
}

// -------------------------------------------------------------------------------------------------

pub(crate) struct Iter<'a> {
    mbox: &'a Mbox,
    idx: usize,
}

impl<'a> Iter<'a> {
    fn new(mbox: &'a Mbox) -> Self {
        Iter { mbox, idx: 0 }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = MessageCursor<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        (self.idx < self.mbox.count()).then(|| {
            let item = MessageCursor::new(&self.mbox.messages[self.idx], &self.mbox.lines);
            self.idx += 1;
            item
        })
    }
}

//pub(crate) struct IterMut<'a> {
//    mbox: &'a mut Mbox,
//    idx: usize,
//}

//impl<'a> IterMut<'a> {
//    fn new(mbox: &'a mut Mbox) -> Self {
//        todo!()
//    }
//}

pub(crate) struct MessageCursor<'a> {
    msg: &'a Message,
    lines: &'a [String],
}

impl<'a> MessageCursor<'a> {
    fn new(msg: &'a Message, lines: &'a [String]) -> Self {
        MessageCursor { msg, lines }
    }

    pub(crate) fn field(&self, field: FieldType) -> Option<&'a str> {
        self.msg
            .field_idcs
            .get(&field)
            .map(|local_idx| self.lines[self.msg.start_idx + local_idx].as_str())
    }

    pub(crate) fn all_lines(&self) -> &'a [String] {
        &self.lines[self.msg.start_idx..self.msg.end_idx]
    }

    pub(crate) fn body_lines(&self) -> Option<&'a [String]> {
        self.msg
            .field_idcs
            .get(&FieldType::Body)
            .map(|body_idx| &self.lines[(self.msg.start_idx + *body_idx)..self.msg.end_idx])
    }
}

// -------------------------------------------------------------------------------------------------

#[derive(Debug)]
struct FieldsParser {
    field_prefixes: Vec<(&'static str, FieldType)>,
}

impl FieldsParser {
    pub(super) fn default() -> Self {
        FieldsParser {
            field_prefixes: vec![
                ("Date: ", FieldType::Date),
                ("From: ", FieldType::From),
                ("Subject: ", FieldType::Subject),
                ("Status: ", FieldType::Status),
            ],
        }
    }

    pub(super) fn gather_fields(&self, lines: &[String]) -> HashMap<FieldType, usize> {
        lines
            .iter()
            .enumerate()
            .fold(HashMap::new(), |mut fields, (idx, line)| {
                if let Some(field) = self
                    .field_prefixes
                    .iter()
                    .find_map(|(prefix, field)| line.starts_with(prefix).then_some(field))
                {
                    fields.insert(*field, idx);
                } else if line.is_empty() && !fields.contains_key(&FieldType::Body) {
                    fields.insert(FieldType::Body, idx + 1);
                }
                fields
            })
    }
}

// -------------------------------------------------------------------------------------------------
