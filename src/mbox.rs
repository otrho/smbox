use std::io;

// -------------------------------------------------------------------------------------------------

pub fn get_mbox_path() -> io::Result<String> {
    match std::env::vars_os().find(|(key, _)| key == "MAIL") {
        None => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "error: Unable to determine mbox path; missing MAIL environment variable.",
        )),
        Some((_, env_value)) => env_value.into_string().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "error: Malformed string in MAIL environment variable.",
            )
        }),
    }
}

// -------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct Message {
    pub start_idx: i64,
    pub end_idx: i64, // Index to line *after* last line in message.
    pub date_idx: i64,
    pub from_idx: i64,
    pub subject_idx: i64,
    pub body_idx: i64,
}

// -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -

struct MessageListUpdater<'a> {
    messages: &'a mut Vec<Message>,
}

impl<'a> MessageListUpdater<'a> {
    fn add_new_message(&mut self, start_idx: i64) {
        self.messages.push(Message {
            start_idx,
            end_idx: 0,
            date_idx: 0,
            from_idx: 0,
            subject_idx: 0,
            body_idx: 0,
        });
    }

    fn update_last_message<F: FnOnce(&mut Message)>(&mut self, f: F) {
        if let Some(last_message) = self.messages.last_mut() {
            f(last_message)
        }
    }
}

// -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -

pub fn parse_mbox(lines: &Vec<String>) -> Vec<Message> {
    let mut messages = Vec::<Message>::new();
    let mut updater = MessageListUpdater {
        messages: &mut messages,
    };
    for (idx, line) in lines.iter().enumerate() {
        let idx = idx as i64;
        if line.starts_with("From ") {
            updater.update_last_message(|msg| msg.end_idx = idx);
            updater.add_new_message(idx);
        } else if line.starts_with("Date: ") {
            updater.update_last_message(|msg| msg.date_idx = idx);
        } else if line.starts_with("From: ") {
            updater.update_last_message(|msg| msg.from_idx = idx);
        } else if line.starts_with("Subject: ") {
            updater.update_last_message(|msg| msg.subject_idx = idx);
        } else if line.is_empty() {
            updater.update_last_message(|msg| {
                if msg.body_idx == 0 {
                    // The body index points to the first line after the empty line which separates
                    // the headers from the body.
                    msg.body_idx = idx + 1
                }
            });
        }
    }
    updater.update_last_message(|msg| msg.end_idx = lines.len() as i64);
    messages
}

// -------------------------------------------------------------------------------------------------
