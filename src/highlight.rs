// -------------------------------------------------------------------------------------------------

type Colour256 = u8;

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Highlighter {
    #[serde(rename = "highlights")]
    ctx_matchers: Vec<HighlightContext>,
    cur_ctx: Option<usize>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct HighlightContext {
    #[serde(with = "serde_regex", rename = "enter")]
    ctx_enter_re: regex::Regex,

    #[serde(with = "serde_regex", default, rename = "exit")]
    ctx_exit_re: Option<regex::Regex>,

    matchers: Vec<HighlightMatcher>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct HighlightMatcher {
    #[serde(with = "serde_regex", rename = "match")]
    re: regex::Regex,
    colour: Colour256,
}

impl Highlighter {
    pub fn get_highlights(&mut self, line: &str) -> Highlights {
        // First check if we're matching a new context.
        if let Some(new_ctx_idx) = self
            .ctx_matchers
            .iter()
            .enumerate()
            .find_map(|(idx, ctx_matcher)| ctx_matcher.ctx_enter_re.is_match(line).then_some(idx))
        {
            // Set the new context and return empty highlights.
            self.cur_ctx = Some(new_ctx_idx);
            return Highlights::default();
        }

        // If we are matching a specific context then use it.
        let mut highlights = Vec::default();
        if let Some(cur_ctx_idx) = self.cur_ctx {
            let ctx_matcher = &self.ctx_matchers[cur_ctx_idx];

            // Firstly check if we're matching this context's exit pattern.
            if ctx_matcher
                .ctx_exit_re
                .iter()
                .any(|exit_re| exit_re.is_match(line))
            {
                // Set the current context to none.
                self.cur_ctx = None;
            } else {
                // Find any matches for this context.
                for HighlightMatcher { re, colour } in &ctx_matcher.matchers {
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

// -------------------------------------------------------------------------------------------------

#[derive(Default)]
pub struct Highlights {
    highlights: Vec<((usize, usize), Colour256)>,
}

impl Highlights {
    pub fn get_colour_at(&self, idx: usize) -> Option<Colour256> {
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
