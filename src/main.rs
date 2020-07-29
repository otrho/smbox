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
//     [ ] Delete.
//
// Keys:
//   Selector:
//     enter   - view in pager
//     j/k     - down/up
//     d/u     - delete/undelete
//     q       - write changes, exit
//     x       - discard changes, exit
//   Pager:
//     space/f - page down
//     b       - page up
//     g/G     - goto top/bottom
//     d/u     - delete/undelete
//     q       - keep changes, return
//     x       - discard changes, return
//
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

// -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -

fn smbox() -> std::io::Result<()> {
    // Read lines as a vector of strings from the mbox path found in $MAIL.
    let lines = read_lines(get_mbox_path()?)?;
    if lines.is_empty() {
        println!("No mail.");
    } else {
        let messages = parse_mbox(&lines);
        run_event_loop(lines, messages)?;
    }

    println!("");
    Ok(())
}

// -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -

use std::io::Write;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

fn run_event_loop(lines: Vec<String>, messages: Vec<Message>) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().into_raw_mode()?;
    write!(stdout, "{}", termion::cursor::Hide)?;

    let mut msg_idx = 0;
    let mut page_idx = 0;
    update_ui(&lines, &messages, &mut stdout, msg_idx, page_idx)?;

    for inp in stdin.keys() {
        match inp? {
            termion::event::Key::Char('q') => break,
            termion::event::Key::Char('j') => {
                msg_idx = std::cmp::min(msg_idx + 1, messages.len() as i64 - 1)
            }
            termion::event::Key::Char('k') => msg_idx = std::cmp::max(msg_idx - 1, 0),
            _ => {}
        }
        update_ui(&lines, &messages, &mut stdout, msg_idx, page_idx)?;
    }
    write!(stdout, "{}", termion::cursor::Show)
}

// -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -

const MAX_SUMMARY_LINES: i64 = 8;
const MIN_MESSAGE_LINES: i64 = 8;

fn update_ui<T>(
    lines: &Vec<String>,
    messages: &Vec<Message>,
    stdout: &mut termion::raw::RawTerminal<T>,
    message_idx: i64,
    page_idx: i64,
) -> std::io::Result<()>
where
    T: std::io::Write,
{
    // Clear the screen.
    write!(stdout, "{}", termion::clear::All)?;

    let (width, height) = termion::terminal_size()?;
    if (height as i64) < MIN_MESSAGE_LINES + 2 {
        // This is a non-fatal error.
        write!(stdout, "Terminal size is too small.")?;
        return Ok(());
    }

    // Work out how big the message summary list is.
    let num_summary_lines = std::cmp::min(messages.len() as i64, MAX_SUMMARY_LINES);
    let top_summary_idx = if message_idx < num_summary_lines {
        0
    } else {
        std::cmp::min(message_idx, messages.len() as i64 - MAX_SUMMARY_LINES)
    };

    // Determine the max length too.
    let max_summary_line_len = width as usize - 3; // len(">> ") == 3.

    // Write the summaries.
    for idx in 0..num_summary_lines {
        let summary_line = &lines[messages[(idx + top_summary_idx) as usize].subject_idx as usize];
        let summary_line = &summary_line[..std::cmp::min(summary_line.len(), max_summary_line_len)];
        if idx + top_summary_idx == message_idx {
            write!(stdout, "{}>> ", termion::cursor::Goto(1, (idx + 1) as u16))?;
        }
        write!(
            stdout,
            "{}{}",
            termion::cursor::Goto(4, (idx + 1) as u16),
            summary_line
        )?;
    }

    // Write the message.  We start them from a 1 line gap after the summary.
    let top_message_line_offs = num_summary_lines;
    let num_message_lines = height as i64 - top_message_line_offs;
    let message = &messages[message_idx as usize];
    for idx in 0..num_message_lines {
        if idx < message.end_idx - message.body_idx {
            let message_line = &lines[(message.body_idx + idx) as usize];
            let message_line = &message_line[..std::cmp::min(message_line.len(), width as usize)];
            write!(
                stdout,
                "{}{}",
                termion::cursor::Goto(1, (top_message_line_offs + 1 + idx) as u16),
                message_line
            )?;
        } else {
            write!(
                stdout,
                "{}~",
                termion::cursor::Goto(1, (top_message_line_offs + 1 + idx) as u16)
            )?;
        }
    }

    stdout.flush()
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
