use crate::{highlight, mbox};

use termion::{input::TermRead, raw::IntoRawMode, screen::IntoAlternateScreen};

use std::io;

const COL_GREY_IDX: u8 = 236;
const COL_GREEN_IDX: u8 = 71;

// -------------------------------------------------------------------------------------------------

pub(crate) fn run(
    messages: mbox::Mbox,
    highlighter: &mut highlight::Highlighter,
) -> io::Result<Option<mbox::Mbox>> {
    let stdout = io::stdout().into_raw_mode()?.into_alternate_screen()?;
    let stdin = io::stdin();

    let mut terminal = tui::Terminal::new(tui::backend::CrosstermBackend::new(stdout))?;
    let mut iface = IfaceState::new(messages);

    iface.headers_state.select(Some(0));
    iface.mark_selected_read();
    iface.draw(&mut terminal, highlighter)?;

    let mut key_events = stdin.keys();
    loop {
        use termion::event::Key;
        match key_events.next().expect("Endless keys...")? {
            Key::Char('q') => break Ok(Some(iface.messages())),
            Key::Char('x') => break Ok(None),
            Key::Char('j') => iface.increment_header_index(1),
            Key::Char('k') => iface.increment_header_index(-1),
            Key::Char(' ') => iface.increment_page(1),
            Key::Char('b') => iface.increment_page(-1),
            Key::Char('g') => iface.first_page(),
            Key::Char('d') => iface.mark_selected_deleted(),
            _ => (),
        }

        iface.mark_selected_read();
        iface.draw(&mut terminal, highlighter)?;
    }
}

// -------------------------------------------------------------------------------------------------

struct IfaceState {
    mbox: mbox::Mbox,
    headers_state: tui::widgets::ListState,
    body_page_idx: i64,
}

// To simplify things a bit we're doubling down on just using a Termion backend.
type IfaceTerminal = tui::terminal::Terminal<
    tui::backend::CrosstermBackend<
        termion::screen::AlternateScreen<termion::raw::RawTerminal<io::Stdout>>,
    >,
>;

impl IfaceState {
    fn new(mbox: mbox::Mbox) -> IfaceState {
        IfaceState {
            mbox,
            headers_state: tui::widgets::ListState::default(),
            body_page_idx: 0,
        }
    }

    fn messages(self) -> mbox::Mbox {
        self.mbox
    }

    fn increment_header_index(&mut self, delta: i64) {
        let idx = self.headers_state.selected().unwrap_or(0) as i64 + delta;
        let idx = std::cmp::max(idx, 0);
        let idx = std::cmp::min(idx, self.mbox.count() as i64 - 1);
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

    fn mark_selected_read(&mut self) {
        if let Some(idx) = self.headers_state.selected() {
            if let Some(msg) = self.mbox.msg_at_mut(idx) {
                msg.set_status(mbox::Status::Read);
            }
        }
    }

    fn mark_selected_deleted(&mut self) {
        if let Some(idx) = self.headers_state.selected() {
            if let Some(msg) = self.mbox.msg_at_mut(idx) {
                msg.set_status(mbox::Status::Deleted);
            }
            self.increment_header_index(1);
        }
    }

    fn draw(
        &mut self,
        terminal: &mut IfaceTerminal,
        highlighter: &mut highlight::Highlighter,
    ) -> io::Result<()> {
        // A little lambda to remove the 'Title:' prefix and to pad/trunc to a fixed width.
        let prepare_field = |line: &str, width: usize| {
            let mut stripped = line.split(": ").nth(1).unwrap_or(line).to_string();
            stripped.truncate(width);
            format!("{:1$}", stripped, width)
        };

        terminal.draw(|frame| {
            // Split the screen, top 25% for message selection menu, bottom 75% for message text.
            let chunks = tui::layout::Layout::default()
                .direction(tui::layout::Direction::Vertical)
                .constraints(
                    [
                        tui::layout::Constraint::Max(self.mbox.count() as u16),
                        tui::layout::Constraint::Length(1),
                        tui::layout::Constraint::Percentage(75),
                    ]
                    .as_ref(),
                )
                .split(frame.size());

            // Gather info from the message headers for the selection menu items.
            let headers: Vec<tui::widgets::ListItem> = self
                .mbox
                .iter()
                .map(|msg| {
                    tui::widgets::ListItem::new(tui::text::Spans::from(vec![
                        tui::text::Span::raw(format!(
                            "{}{} ",
                            if msg.has_status(mbox::Status::Deleted) {
                                "D"
                            } else {
                                " "
                            },
                            if msg.has_status(mbox::Status::Read) {
                                " "
                            } else if msg.has_status(mbox::Status::NonRecent) {
                                "U"
                            } else {
                                "N"
                            },
                        )),
                        // Make the date 25 chars, the from field 40 and the subject can be
                        // longer.
                        tui::text::Span::raw(prepare_field(
                            msg.field(mbox::FieldType::Date)
                                .map(|line| {
                                    // Truncate the date string.  We're expecting it in the form
                                    // 'Fri, 4 Sep 2020 11:44:49 +1000 (AEST)' and we'll just cut
                                    // off the TZ stuff.
                                    line.split(" +").next().unwrap_or(line)
                                })
                                .unwrap_or("???"),
                            25,
                        )),
                        tui::text::Span::raw(" | "),
                        tui::text::Span::raw(prepare_field(
                            msg.field(mbox::FieldType::From).unwrap_or("???"),
                            40,
                        )),
                        tui::text::Span::raw(" | "),
                        tui::text::Span::raw(prepare_field(
                            msg.field(mbox::FieldType::Subject).unwrap_or("???"),
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
                        .bg(tui::style::Color::Indexed(COL_GREY_IDX)),
                );
            frame.render_stateful_widget(headers, chunks[0], &mut self.headers_state);

            // Add a little infomational divider between the headers and the message body.
            let info_text = format!(
                " --- {}/{} ---",
                self.headers_state
                    .selected()
                    .map(|n| (n + 1).to_string())
                    .unwrap_or_else(|| "??".to_string()),
                self.mbox.count(),
            );
            frame.render_widget(
                tui::widgets::Paragraph::new(tui::text::Span::raw(info_text)).style(
                    tui::style::Style::default().fg(tui::style::Color::Indexed(COL_GREEN_IDX)),
                ),
                chunks[1],
            );

            // Put the message lines into a paragraph for the bottom window.
            let message_text: Vec<tui::text::Spans> = self
                .mbox
                .msg_at(self.headers_state.selected().unwrap_or(0))
                .map(|msg| {
                    msg.body_lines()
                        .unwrap_or(&[])
                        .iter()
                        .map(|line| highlighted_line(highlighter, line))
                        .collect()
                })
                .unwrap_or_default();

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
        })?;
        Ok(())
    }
}

// -------------------------------------------------------------------------------------------------

fn highlighted_line<'a>(
    highlighter: &mut highlight::Highlighter,
    line: &'a str,
) -> tui::text::Spans<'a> {
    let mut spans = Vec::<tui::text::Span>::new();
    let mut save_span = |start, end, colour| match colour {
        None => {
            spans.push(tui::text::Span::raw(&line[start..end]));
        }
        Some(colour_idx) => {
            spans.push(tui::text::Span::styled(
                &line[start..end],
                tui::style::Style::default().fg(tui::style::Color::Indexed(colour_idx)),
            ));
        }
    };

    let highlights = highlighter.get_highlights(line);
    let (final_idx, final_colour) = (0..line.len()).fold(
        (0, highlights.get_colour_at(0)),
        |(start, cur_colour), ch_idx| {
            let next_colour = highlights.get_colour_at(ch_idx);
            if next_colour == cur_colour {
                (start, cur_colour)
            } else {
                save_span(start, ch_idx, cur_colour);
                (ch_idx, next_colour)
            }
        },
    );
    save_span(final_idx, line.len(), final_colour);

    tui::text::Spans::from(spans)
}

// -------------------------------------------------------------------------------------------------
