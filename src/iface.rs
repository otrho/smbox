// -------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub enum Action {
    DeleteMessage(i64),
}

// -------------------------------------------------------------------------------------------------

use termion::input::TermRead;
use termion::raw::IntoRawMode;

pub fn run(
    lines: &Vec<String>,
    messages: &Vec<super::mbox::Message>,
) -> std::io::Result<Vec<Action>> {
    let stdout = std::io::stdout().into_raw_mode()?;
    let stdout = termion::screen::AlternateScreen::from(stdout);
    let stdin = std::io::stdin();

    let mut terminal = tui::Terminal::new(tui::backend::TermionBackend::new(stdout))?;
    let mut iface = IfaceState::new(lines, messages);

    iface.headers_state.select(Some(0));
    iface.draw(&mut terminal)?;

    for inp_key in stdin.keys() {
        match inp_key? {
            termion::event::Key::Char('q') => break,
            termion::event::Key::Char('x') => {
                iface.clear_deleted_messages();
                break;
            }
            termion::event::Key::Char('j') => iface.increment_header_index(1),
            termion::event::Key::Char('k') => iface.increment_header_index(-1),
            termion::event::Key::Char(' ') => iface.increment_page(1),
            termion::event::Key::Char('b') => iface.increment_page(-1),
            termion::event::Key::Char('g') => iface.first_page(),
            termion::event::Key::Char('d') => iface.delete_selected(),
            _ => (),
        }

        iface.draw(&mut terminal)?;
    }

    Ok(iface
        .deleted_messages_iter()
        .map(|&idx| Action::DeleteMessage(idx))
        .collect())
}

// -------------------------------------------------------------------------------------------------

struct IfaceState<'a> {
    lines: &'a Vec<String>,
    messages: &'a Vec<super::mbox::Message>,
    headers_state: tui::widgets::ListState,
    body_page_idx: i64,
    deleted_messages: std::collections::BTreeSet<i64>,
}

// To simplify things a bit we're doubling down on just using a Termion backend.
type IfaceTerminal = tui::terminal::Terminal<
    tui::backend::TermionBackend<
        termion::screen::AlternateScreen<termion::raw::RawTerminal<std::io::Stdout>>,
    >,
>;

impl<'a> IfaceState<'a> {
    fn new(lines: &'a Vec<String>, messages: &'a Vec<super::mbox::Message>) -> IfaceState<'a> {
        IfaceState {
            lines,
            messages,
            headers_state: tui::widgets::ListState::default(),
            body_page_idx: 0,
            deleted_messages: std::collections::BTreeSet::new(),
        }
    }

    fn increment_header_index(&mut self, delta: i64) {
        let idx = self.headers_state.selected().unwrap_or(0) as i64 + delta;
        let idx = std::cmp::max(idx, 0);
        let idx = std::cmp::min(idx, self.messages.len() as i64 - 1);
        self.headers_state.select(Some(idx as usize));
        self.body_page_idx = 0;
    }

    fn increment_page(&mut self, delta: i64) {
        // The bounds check (and remedy) is performed below when drawing the message.
        self.body_page_idx += delta;
    }

    fn first_page(&mut self) {
        self.body_page_idx = 0;
    }

    fn delete_selected(&mut self) {
        if let Some(idx) = self.headers_state.selected() {
            self.deleted_messages.insert(idx as i64);
            self.increment_header_index(1);
        }
    }

    fn clear_deleted_messages(&mut self) {
        self.deleted_messages.clear();
    }

    fn deleted_messages_iter(&self) -> impl Iterator<Item = &i64> {
        self.deleted_messages.iter()
    }

    fn draw(&mut self, terminal: &mut IfaceTerminal) -> std::io::Result<()> {
        // A little lambda to remove the 'Title:' prefix and to pad/trunc to a fixed width.
        let prepare_field = |line: &str, width: usize| {
            let mut stripped = line.split(": ").nth(1).unwrap_or(&line).to_string();
            stripped.truncate(width);
            format!("{:1$}", stripped, width)
        };

        // Another little lambda to truncate the date string.  We're expecting it in the form
        // 'Fri, 4 Sep 2020 11:44:49 +1000 (AEST)' and we'll just cut off the TZ stuff.
        let reformat_date = |line: &'a str| line.split(" +").nth(0).unwrap_or(&line);

        terminal.draw(|frame| {
            // Split the screen, top 25% for message selection menu, bottom 75% for message text.
            let chunks = tui::layout::Layout::default()
                .direction(tui::layout::Direction::Vertical)
                .constraints(
                    [
                        tui::layout::Constraint::Max(self.messages.len() as u16),
                        tui::layout::Constraint::Length(1),
                        tui::layout::Constraint::Percentage(75),
                    ]
                    .as_ref(),
                )
                .split(frame.size());

            // Gather info from the message headers for the selection menu items.
            let headers: Vec<tui::widgets::ListItem> = self
                .messages
                .iter()
                .enumerate()
                .map(|(idx, msg)| {
                    tui::widgets::ListItem::new(tui::text::Spans::from(vec![
                        tui::text::Span::raw(if self.deleted_messages.contains(&(idx as i64)) {
                            "D "
                        } else {
                            "  "
                        }),
                        // Make the date 20 chars, the from field 40 and the subject can be
                        // longer.
                        tui::text::Span::raw(prepare_field(
                            &reformat_date(&self.lines[msg.date_idx as usize]),
                            25,
                        )),
                        tui::text::Span::raw(" | "),
                        tui::text::Span::raw(prepare_field(&self.lines[msg.from_idx as usize], 40)),
                        tui::text::Span::raw(" | "),
                        tui::text::Span::raw(prepare_field(
                            &self.lines[msg.subject_idx as usize],
                            80, // XXX Should be at least to screen width.
                        )),
                    ]))
                })
                .collect();
            let headers = tui::widgets::List::new(headers)
                .highlight_symbol("> ")
                .highlight_style(
                    tui::style::Style::default()
                        .fg(tui::style::Color::White)
                        .bg(tui::style::Color::Indexed(236)), // Grey.
                );
            frame.render_stateful_widget(headers, chunks[0], &mut self.headers_state);

            // Add a little infomational divider between the headers and the message body.
            let info_text = format!(
                " --- {}/{} ---",
                self.headers_state
                    .selected()
                    .map(|n| (n + 1).to_string())
                    .unwrap_or("??".to_string()),
                self.messages.len(),
            );
            frame.render_widget(
                tui::widgets::Paragraph::new(tui::text::Span::raw(info_text))
                    .style(tui::style::Style::default().fg(tui::style::Color::Indexed(71))), // Green.
                chunks[1],
            );

            // Put the message lines into a paragraph for the bottom window.
            let message = &self.messages[self.headers_state.selected().unwrap_or(0)];
            let mut message_text = Vec::<tui::text::Spans>::new();
            for message_line_idx in message.body_idx..message.end_idx {
                message_text.push(tui::text::Spans::from(
                    self.lines[message_line_idx as usize].clone(),
                ));
            }

            // It doesn't seem to be possible to get the size of a Layout -- we'd like to choose a
            // page based on the lower chunk size.  Instead we'll just go with 75% of the height.
            let page_size = frame.size().height * 3 / 4;
            let max_page_idx = message_text.len() / page_size as usize;
            self.body_page_idx = std::cmp::max(self.body_page_idx, 0);
            self.body_page_idx = std::cmp::min(self.body_page_idx, max_page_idx as i64);
            frame.render_widget(
                tui::widgets::Paragraph::new(message_text)
                    .scroll(((self.body_page_idx as u16 * page_size), 0)),
                chunks[2],
            );
        })
    }
}

// -------------------------------------------------------------------------------------------------
