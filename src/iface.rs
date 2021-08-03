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

    let mut highlight_ctx = Highlighter {
        ctx_matchers: vec![
            HighlightContext {
                ctx_re: regex::Regex::new(r"login failures:$").unwrap(),
                ctx_id: "login failures".to_owned(),
                matchers: vec![
                    (
                        regex::Regex::new(r"d......d").expect("Failed to compile RE."),
                        107_u8,
                    ),
                    (regex::Regex::new(r" p[a-z]+ ").unwrap(), 167_u8),
                    (
                        regex::Regex::new(r"Bad protocol version.*(port)").unwrap(),
                        140_u8,
                    ),
                    (regex::Regex::new(r"protocol version").unwrap(), 90_u8),
                    (regex::Regex::new(r"Bad protocol").unwrap(), 100_u8),
                ],
            },
            HighlightContext {
                ctx_re: regex::Regex::new(r"denied packets:$").unwrap(),
                ctx_id: "denied packets".to_owned(),
                matchers: vec![(regex::Regex::new(r" p[a-z]+ ").unwrap(), 90_u8)],
            },
        ],
        ctx_exit_re: Some(regex::Regex::new(r"^$").unwrap()),
        ctx: None,
    };

    iface.headers_state.select(Some(0));
    iface.draw(&mut terminal, &mut highlight_ctx)?;

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

        iface.draw(&mut terminal, &mut highlight_ctx)?;
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

    fn draw(
        &mut self,
        terminal: &mut IfaceTerminal,
        highlighter: &mut Highlighter,
    ) -> std::io::Result<()> {
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
            let message_text: Vec<tui::text::Spans> = (message.body_idx..message.end_idx)
                .map(|line_idx| highlighted_line(highlighter, &self.lines[line_idx as usize]))
                .collect();

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

fn highlighted_line<'a>(highlighter: &mut Highlighter, line: &'a str) -> tui::text::Spans<'a> {
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

struct Highlighter {
    ctx_matchers: Vec<HighlightContext>,
    ctx_exit_re: Option<regex::Regex>,
    ctx: Option<String>,
}

struct HighlightContext {
    ctx_re: regex::Regex,
    ctx_id: String,
    matchers: Vec<(regex::Regex, Colour256)>,
}

type Colour256 = u8;

impl Highlighter {
    fn get_highlights(&mut self, line: &str) -> Highlights {
        if let Some(exit_re) = &self.ctx_exit_re {
            if exit_re.is_match(line) {
                self.ctx = None;
            }
        }

        let new_ctx = self.ctx_matchers.iter().fold(None, |new_ctx, ctx_matcher| {
            if ctx_matcher.ctx_re.is_match(line) {
                Some(&ctx_matcher.ctx_id)
            } else {
                new_ctx
            }
        });
        if new_ctx.is_some() {
            self.ctx = new_ctx.map(|id| id.clone());
            return Highlights {
                highlights: Vec::new(),
            };
        }

        let mut highlights = Vec::new();
        for ctx_matcher in &self.ctx_matchers {
            if self
                .ctx
                .as_ref()
                .map_or(false, |cur_id| cur_id == &ctx_matcher.ctx_id)
            {
                for (re, colour) in &ctx_matcher.matchers {
                    if let Some(caps) = re.captures(line) {
                        let mtch = caps
                            .get(if caps.len() == 1 { 0 } else { 1 })
                            .expect("BUG! `caps` is guaranteed to have at least one match.");
                        highlights.push(((mtch.start(), mtch.end()), *colour));
                    }
                }
            }
        }
        Highlights { highlights }
    }
}

struct Highlights {
    highlights: Vec<((usize, usize), Colour256)>,
}

impl Highlights {
    fn get_colour_at(&self, idx: usize) -> Option<Colour256> {
        self.highlights
            .iter()
            .fold(None, |prev, ((start, end), colour)| {
                if idx >= *start && idx < *end {
                    Some(*colour)
                } else {
                    prev
                }
            })
    }
}

// -------------------------------------------------------------------------------------------------
