use std::io::BufRead;

// -------------------------------------------------------------------------------------------------
// [X] Find mbox.
// [X] Read all lines.
// [ ] Divide into messages:
//     [X] '^From' divider.
//     [X] Useful headers, especially 'Subject:'.
//     [X] Body.
// [ ] UI:
//     [ ] Message selector, headers summary.
//     [ ] Show messages (via pager?)
//     [ ] Colours.
// -------------------------------------------------------------------------------------------------

fn main() {
    std::process::exit(match smbox() {
        Ok(_) => 0,
        Err(err) => {
            println!("{}", err);
            1
        }
    });
}

fn smbox() -> std::io::Result<()> {
    // Read lines as a vector of strings from the mbox path found in $MAIL.
    let lines = read_lines(get_mbox_path()?)?;
    if lines.is_empty() {
        println!("No mail.");
    } else {
        let messages = parse_mbox(&lines);
        for message in messages {
            println!("{:?}", message)
        }
    }

    Ok(())
}

// -------------------------------------------------------------------------------------------------

fn read_lines(path: String) -> std::io::Result<Vec<String>> {
    let reader = std::io::BufReader::new(std::fs::File::open(path)?);
    let mut lines = Vec::<String>::new();
    for line in reader.lines() {
        lines.push(line?);
    }
    Ok(lines)
}

// -------------------------------------------------------------------------------------------------

fn get_mbox_path() -> std::io::Result<String> {
    match std::env::vars_os().find(|(key, _)| key == "MAIL") {
        None => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "error: Unable to determine mbox path; missing MAIL environment variable.",
        )),
        Some((_, env_value)) => env_value.into_string().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "error: Malformed string in MAIL environment variable.",
            )
        }),
    }
}

// -------------------------------------------------------------------------------------------------

#[derive(Debug)]
struct Message {
    start_idx: i64,
    end_idx: i64, // Index to line *after* last line in message.
    date_idx: i64,
    from_idx: i64,
    subject_idx: i64,
    body_idx: i64,
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

fn parse_mbox(lines: &Vec<String>) -> Vec<Message> {
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
                    msg.body_idx = idx
                }
            });
        }
    }
    updater.update_last_message(|msg| msg.end_idx = lines.len() as i64);
    messages
}

// -------------------------------------------------------------------------------------------------
