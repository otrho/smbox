use std::io::BufRead;

// -------------------------------------------------------------------------------------------------
// [X] Find mbox.
// [X] Read all lines.
// [X] Divide into messages:
//     [X] '^From' divider.
//     [X] Useful headers, especially 'Subject:'.
//     [X] Body.
// [ ] UI:
//     [X] Message selector, headers summary.
//     [X] Show messages.
//     [ ] Improved selector info (date, from, subject, etc.)
//     [ ] Show headers in pager.
//     [ ] Colours:
//         [ ] For message selector.
//         [ ] Today's mail highlighted.
//         [ ] Divider, with n/m shown.
//         [ ] Deleted messages grey in summary.
//     [ ] Delete.
//     [ ] Highlight filters.
//     [ ] Show all headers toggle.
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

    println!();
    Ok(())
}

// -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -

use std::io::Write;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

fn run_event_loop(lines: Vec<String>, messages: Vec<Message>) -> std::io::Result<()> {
    let mut stdout = std::io::stdout().into_raw_mode()?;
    let mut tui = MessagesModel::new(lines, messages, &mut stdout);
    tui.show()?;

    let stdin = std::io::stdin();
    for inp in stdin.keys() {
        match inp? {
            termion::event::Key::Char('q') => break,
            termion::event::Key::Char('j') => tui.increment_selected(),
            termion::event::Key::Char('k') => tui.decrement_selected(),
            termion::event::Key::Char(' ') | termion::event::Key::Char('f') => tui.page_view_down()?,
            termion::event::Key::Char('b') => tui.page_view_up()?,
            _ => {}
        }
        tui.show()?;
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

fn parse_mbox(lines: &[String]) -> Vec<Message> {
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

struct MessagesModel<'a, T: std::io::Write> {
    lines: Vec<String>,
    messages: Vec<Message>,
    stdout: &'a mut termion::raw::RawTerminal<T>,
    selected_message_idx: i64,
    viewed_top_line_idx: i64,
}

impl<'a, T: std::io::Write> Drop for MessagesModel<'a, T> {
    fn drop(&mut self) {
        // Show the cursor again.
        write!(self.stdout, "{}", termion::cursor::Show).unwrap();
    }
}

const MAX_SUMMARY_LINES: i64 = 8;
const MIN_MESSAGE_LINES: i64 = 8;

impl<'a, T: std::io::Write> MessagesModel<'a, T> {
    fn new(
        lines: Vec<String>,
        messages: Vec<Message>,
        stdout: &mut termion::raw::RawTerminal<T>,
    ) -> MessagesModel<T> {
        // Hide the cursor on construction.
        write!(stdout, "{}", termion::cursor::Hide).unwrap();

        MessagesModel {
            lines,
            messages,
            stdout: stdout,
            selected_message_idx: 0,
            viewed_top_line_idx: 0,
        }
    }

    fn increment_selected(&mut self) {
        self.selected_message_idx =
            std::cmp::min(self.selected_message_idx + 1, self.lines.len() as i64 - 1);
        self.viewed_top_line_idx = 0
    }

    fn decrement_selected(&mut self) {
        self.selected_message_idx = std::cmp::max(self.selected_message_idx - 1, 0);
        self.viewed_top_line_idx = 0
    }

    fn page_view_down(&mut self) -> std::io::Result<()> {
        let (_, _, num_message_lines) = self.get_tui_dimensions()?;
        let message = &self.messages[self.selected_message_idx as usize];
        if self.viewed_top_line_idx + num_message_lines < message.end_idx {
            self.viewed_top_line_idx += num_message_lines
        }
        Ok(())
    }

    fn page_view_up(&mut self) -> std::io::Result<()> {
        let (_, _, num_message_lines) = self.get_tui_dimensions()?;
        if self.viewed_top_line_idx - num_message_lines >= 0 {
            self.viewed_top_line_idx -= num_message_lines
        }
        Ok(())
    }

    fn get_tui_dimensions(&self) -> std::io::Result<(i64, i64, i64)> {
        // Return the width, summary height and view height.  We assume a single line gap between
        // the summary and the message view.
        let (width, height) = termion::terminal_size()?;
        if (height as i64) < MIN_MESSAGE_LINES + 2 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Terminal size is too small.",
            ));
        }

        let num_summary_lines = std::cmp::min(self.messages.len() as i64, MAX_SUMMARY_LINES);
        let num_message_lines = height as i64 - num_summary_lines - 1;
        Ok((width as i64, num_summary_lines, num_message_lines))
    }

    fn show(&mut self) -> std::io::Result<()> {
        // Clear the screen.
        write!(self.stdout, "{}", termion::clear::All)?;

        // Work out how big the message summary list is.
        let (width, num_summary_lines, num_message_lines) = self.get_tui_dimensions()?;
        let top_summary_idx = if self.selected_message_idx < num_summary_lines {
            0
        } else {
            std::cmp::min(
                self.selected_message_idx,
                self.messages.len() as i64 - MAX_SUMMARY_LINES,
            )
        };

        // Determine the max length too.
        let max_summary_line_len = width as usize - 3; // len(">> ") == 3.

        // Write the summaries.
        for idx in 0..num_summary_lines {
            let summary_line =
                &self.lines[self.messages[(idx + top_summary_idx) as usize].subject_idx as usize];
            let summary_line =
                &summary_line[..std::cmp::min(summary_line.len(), max_summary_line_len)];
            if idx + top_summary_idx == self.selected_message_idx {
                write!(
                    self.stdout,
                    "{}>> ",
                    termion::cursor::Goto(1, (idx + 1) as u16)
                )?;
            }
            write!(
                self.stdout,
                "{}{}",
                termion::cursor::Goto(4, (idx + 1) as u16),
                summary_line
            )?;
        }

        // Write the message.  We start them from a 1 line gap after the summary.
        let top_message_line_offs = num_summary_lines + 1;
        let message = &self.messages[self.selected_message_idx as usize];
        let top_message_line_idx = std::cmp::max(
            message.body_idx,
            std::cmp::min(
                message.body_idx + self.viewed_top_line_idx,
                message.end_idx - num_message_lines,
            ),
        );
        for idx in 0..num_message_lines {
            if top_message_line_idx + idx < message.end_idx {
                let message_line = &self.lines[(top_message_line_idx + idx) as usize];
                let message_line =
                    &message_line[..std::cmp::min(message_line.len(), width as usize)];
                write!(
                    self.stdout,
                    "{}{}",
                    termion::cursor::Goto(1, (top_message_line_offs + 1 + idx) as u16),
                    message_line
                )?;
            } else {
                write!(
                    self.stdout,
                    "{}~",
                    termion::cursor::Goto(1, (top_message_line_offs + 1 + idx) as u16)
                )?;
            }
        }

        self.stdout.flush()
    }
}

// -------------------------------------------------------------------------------------------------
