use anyhow::Context;
use fxhash::FxHashMap;

// -------------------------------------------------------------------------------------------------

pub fn get_mbox_path() -> anyhow::Result<String> {
    std::env::var_os("MAIL")
        .context("Unable to determine mbox path; missing MAIL environment variable.")
        .and_then(|env_value| {
            env_value
                .into_string()
                .map_err(|_| anyhow::anyhow!("Malformed string in MAIL environment variable.",))
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

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum Status {
    Read,
    NonRecent,
    Deleted,
}

impl Status {
    pub(crate) fn field_char(&self) -> char {
        match self {
            Status::Read => 'R',
            Status::NonRecent => 'O',
            Status::Deleted => 'D',
        }
    }
}

#[derive(Debug)]
pub(crate) struct Message {
    lines: Vec<String>,
    field_idcs: FxHashMap<FieldType, usize>,
}

impl Message {
    fn new(lines: Vec<String>, field_idcs: FxHashMap<FieldType, usize>) -> Self {
        Message { lines, field_idcs }
    }

    pub(crate) fn field(&self, field: FieldType) -> Option<&str> {
        self.field_idcs
            .get(&field)
            .map(|idx| self.lines[*idx].as_str())
    }

    pub(crate) fn has_status(&self, status: Status) -> bool {
        self.field(FieldType::Status)
            .map(|line| line.contains(status.field_char()))
            .unwrap_or(false)
    }

    pub(crate) fn set_status(&mut self, status: Status) {
        match self.field_idcs.get(&FieldType::Status) {
            Some(idx) => {
                // Append the status char if it isn't already there.
                let line = &mut self.lines[*idx];
                if !line.contains(status.field_char()) {
                    line.push(status.field_char());
                }
            }
            None => {
                // Create a new status field line and insert it.  We put it at the end of the
                // headers, right before the blank line before the body.
                if let Some(body_idx) = self.field_idcs.get_mut(&FieldType::Body) {
                    let status_idx = *body_idx - 1;
                    *body_idx += 1;
                    self.lines
                        .insert(status_idx, format!("Status: {}", status.field_char()));
                    self.field_idcs.insert(FieldType::Status, status_idx);
                }
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn unset_status(&mut self, status: Status) {
        if let Some(idx) = self.field_idcs.get(&FieldType::Status) {
            // The status field has a 'Status: ' prefix, but thankfully none of the field chars (R,
            // O, D) are in it, so we can filter the entire line.
            self.lines[*idx].retain(|ch| ch != status.field_char());
        }
    }

    pub(crate) fn all_lines(&self) -> &[String] {
        &self.lines
    }

    pub(crate) fn body_lines(&self) -> Option<&[String]> {
        self.field_idcs
            .get(&FieldType::Body)
            .map(|body_idx| &self.lines[*body_idx..])
    }
}

// -------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct Mbox {
    messages: Vec<Message>,
}

impl std::iter::FromIterator<String> for Mbox {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        let field_parser = FieldsParser::default();

        let mut messages = Vec::new();
        let mut save_message = |lines: Vec<String>| {
            if !lines.is_empty() {
                let fields = field_parser.gather_fields(&lines);
                messages.push(Message::new(lines, fields));
            }
        };

        let last_lines = iter.into_iter().fold(Vec::new(), |mut lines, line| {
            if line.starts_with("From ") {
                // Save the old message.
                save_message(lines);

                // Start saving new set of lines.
                vec![line]
            } else {
                lines.push(line);
                lines
            }
        });
        save_message(last_lines);

        Mbox { messages }
    }
}

impl Mbox {
    pub(crate) fn count(&self) -> usize {
        self.messages.len()
    }

    pub(crate) fn msg_at(&self, idx: usize) -> Option<&Message> {
        self.messages.get(idx)
    }

    pub(crate) fn msg_at_mut(&mut self, idx: usize) -> Option<&mut Message> {
        self.messages.get_mut(idx)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Message> {
        self.messages.iter()
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut Message> {
        self.messages.iter_mut()
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

    pub(super) fn gather_fields(&self, lines: &[String]) -> FxHashMap<FieldType, usize> {
        lines
            .iter()
            .enumerate()
            .fold(FxHashMap::default(), |mut fields, (idx, line)| {
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
