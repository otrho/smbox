use crate::{
    highlight::{Highlight, HighlightConfig},
    mbox,
};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    prelude::*,
    symbols::scrollbar,
    widgets::{
        Block, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Widget, Wrap,
    },
    DefaultTerminal,
};

pub(crate) fn run(
    messages: mbox::Mbox,
    highlighter: HighlightConfig,
) -> anyhow::Result<Option<mbox::Mbox>> {
    let mut terminal = ratatui::init();
    let result = IfaceState::new(messages, highlighter).run(&mut terminal);
    ratatui::restore();
    result
}

struct IfaceState {
    mbox: mbox::Mbox,
    highlight_config: HighlightConfig,
    finished: Option<ExitType>,
    selector: ListState,
    scrollbar: ScrollbarState,
    scroll_count: usize,
    wrap: bool,
}

enum ExitType {
    NoChange,
    Update,
}

const SCROLL_LINES_COUNT: usize = 24;

impl IfaceState {
    fn new(mbox: mbox::Mbox, highlighter: HighlightConfig) -> IfaceState {
        IfaceState {
            mbox,
            highlight_config: highlighter,
            finished: None,
            selector: Default::default(),
            scrollbar: Default::default(),
            scroll_count: 0,
            wrap: false,
        }
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<Option<mbox::Mbox>> {
        self.selector.select_first();
        self.set_selected_status(mbox::Status::Read);

        while self.finished.is_none() {
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
            self.handle_events()?;
        }

        match self.finished {
            Some(ExitType::Update) => Ok(Some(self.mbox)),
            Some(ExitType::NoChange) | None => Ok(None),
        }
    }

    fn handle_events(&mut self) -> anyhow::Result<()> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('q') => {
                    self.finished = Some(ExitType::Update);
                }
                KeyCode::Char('x') => {
                    self.finished = Some(ExitType::NoChange);
                }

                KeyCode::Char('j') => {
                    self.select_next();
                    self.set_selected_status(mbox::Status::Read);
                }
                KeyCode::Char('k') => {
                    self.select_prev();
                    self.set_selected_status(mbox::Status::Read);
                }

                KeyCode::Char(' ') => {
                    self.scroll_count = self.scroll_count.saturating_add(SCROLL_LINES_COUNT);
                }
                KeyCode::Char('b') => {
                    self.scroll_count = self.scroll_count.saturating_sub(SCROLL_LINES_COUNT);
                }
                KeyCode::Char('g') => {
                    self.scroll_count = 0;
                }

                KeyCode::Char('s') => {
                    self.wrap = !self.wrap;
                }

                KeyCode::Char('d') => {
                    self.set_selected_status(mbox::Status::Deleted);
                    self.select_next();
                    self.set_selected_status(mbox::Status::Read);
                }

                _ => (),
            },

            _ => {}
        }

        Ok(())
    }

    fn select_next(&mut self) {
        self.selector.select_next();
        self.scroll_count = 0;
    }

    fn select_prev(&mut self) {
        self.selector.select_previous();
        self.scroll_count = 0;
    }

    fn set_selected_status(&mut self, status: mbox::Status) {
        if let Some(idx) = self.selector.selected() {
            if let Some(msg) = self.mbox.msg_at_mut(idx) {
                msg.set_status(status);
            }
        }
    }

    fn render_selector_list(&mut self, area: Rect, buf: &mut Buffer) {
        // A little util to remove the 'Title:' prefix and to pad/trunc to a fixed width.
        let prepare_field = |line: &str, width: usize| {
            let mut stripped = line.split(": ").nth(1).unwrap_or(line).to_string();
            stripped.truncate(width);
            format!("{:1$}", stripped, width)
        };

        // Gather info from the message headers for the selection list items.
        let items = self
            .mbox
            .iter()
            .map(|msg| {
                let del_status = if msg.has_status(mbox::Status::Deleted) {
                    "D"
                } else {
                    " "
                };

                let read_status = if msg.has_status(mbox::Status::Read) {
                    " "
                } else if msg.has_status(mbox::Status::NonRecent) {
                    "U"
                } else {
                    "N"
                };

                // Make the date 25 chars, the from field 40 and the subject can be longer.
                // XXX: This could probably be done with `Text` manips and could then have styling.
                let date = prepare_field(
                    msg.field(mbox::FieldType::Date)
                        .map(|line| {
                            // Truncate the date string.  We're expecting it in the form
                            // 'Fri, 4 Sep 2020 11:44:49 +1000 (AEST)' and we'll just cut
                            // off the TZ stuff.
                            line.split(" +").next().unwrap_or(line)
                        })
                        .unwrap_or("???"),
                    25,
                );
                let from = prepare_field(msg.field(mbox::FieldType::From).unwrap_or("???"), 40);
                let subject =
                    prepare_field(msg.field(mbox::FieldType::Subject).unwrap_or("???"), 80);

                ListItem::new(format!(
                    "{del_status}{read_status} {date} | {from} | {subject}"
                ))
            })
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(Block::new())
            .highlight_style(Style::default().fg(Color::Black).bg(Color::DarkGray));

        StatefulWidget::render(list, area, buf, &mut self.selector);
    }

    fn render_body_text(&mut self, area: Rect, buf: &mut Buffer) {
        let selected_idx = self.selector.selected();

        let title = if let Some(idx) = selected_idx {
            format!("{}/{}", idx + 1, self.mbox.count())
        } else {
            format!("?/{}", self.mbox.count())
        };

        // XXX: There's a lot of copying going on here.  Ideally we'd be returning `&str` from the
        // mbox and highlighter and using the mbox lifetime everywhere.
        let mut highlighter = self.highlight_config.highlighter();
        let highlight_lines = |lines: &[String]| {
            lines
                .iter()
                .map(|line| {
                    let highlights = highlighter.next_highlights(line);
                    if highlights.is_empty() {
                        Line::from(line.clone())
                    } else {
                        let mut spans = Vec::default();

                        let first_highlight_begin = highlights[0].begin;
                        if first_highlight_begin > 0 {
                            // Start of line is not highlighted.
                            spans.push(Span::raw(line[0..first_highlight_begin].to_string()));
                        }

                        for Highlight { begin, end, colour } in &highlights {
                            spans.push(Span::styled(
                                line[*begin..*end].to_string(),
                                Color::Indexed(*colour),
                            ));
                        }

                        let last_highlight_end = highlights.last().unwrap().end;
                        if last_highlight_end < line.len() {
                            // End of line is not highlighted.
                            spans.push(Span::raw(line[last_highlight_end..].to_string()));
                        }

                        Line::from(spans)
                    }
                })
                .collect::<Vec<Line>>()
        };

        let message_lines = self
            .mbox
            .msg_at(selected_idx.unwrap_or(0))
            .and_then(|msg| msg.body_lines())
            .map(highlight_lines)
            .unwrap_or_default();

        self.scrollbar = self
            .scrollbar
            .content_length(message_lines.len())
            .position(self.scroll_count);

        let body = Paragraph::new(message_lines)
            .block(Block::bordered().title(Line::styled(title, Style::new().fg(Color::Green))))
            .scroll((self.scroll_count as u16, 0));

        let wrapped_body = if self.wrap {
            body.wrap(Wrap { trim: false })
        } else {
            body
        };

        Widget::render(wrapped_body, area, buf);

        StatefulWidget::render(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .symbols(scrollbar::VERTICAL)
                .begin_symbol(None)
                .track_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            buf,
            &mut self.scrollbar,
        );
    }
}

impl Widget for &mut IfaceState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Render the list with an entry for every message, but at most 10 entries.
        let [list_area, text_area] = Layout::vertical([
            Constraint::Length(self.mbox.count().min(10) as u16),
            Constraint::Min(1),
        ])
        .areas(area);

        self.render_selector_list(list_area, buf);
        self.render_body_text(text_area, buf);
    }
}
