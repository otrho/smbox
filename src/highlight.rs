type Colour256 = u8;

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
pub(crate) struct HighlightConfig {
    #[serde(rename = "highlights")]
    ctx_matches: Vec<HighlightContext>,
}

impl<'h> HighlightConfig {
    pub(crate) fn highlighter(&'h self) -> Highlighter<'h> {
        Highlighter {
            config: self,
            cur_ctx: None,
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct HighlightContext {
    #[serde(with = "serde_regex", rename = "enter")]
    ctx_enter_re: regex::Regex,

    #[serde(with = "serde_regex", default, rename = "exit")]
    ctx_exit_re: Option<regex::Regex>,

    matches: Vec<HighlightMatch>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct HighlightMatch {
    #[serde(with = "serde_regex", rename = "match")]
    re: regex::Regex,
    colour: Colour256,
}

#[derive(Default)]
pub(crate) struct Highlight {
    pub(crate) begin: usize,
    pub(crate) end: usize,
    pub(crate) colour: Colour256,
}

pub(crate) struct Highlighter<'h> {
    config: &'h HighlightConfig,
    cur_ctx: Option<usize>,
}

impl<'h> Highlighter<'h> {
    pub(crate) fn next_highlights(&mut self, next_line: &str) -> Vec<Highlight> {
        // First check if we're matching a new context.
        if let Some(new_ctx_idx) =
            self.config
                .ctx_matches
                .iter()
                .enumerate()
                .find_map(|(idx, ctx_matcher)| {
                    ctx_matcher.ctx_enter_re.is_match(next_line).then_some(idx)
                })
        {
            // Set the new context and return empty highlights.
            self.cur_ctx = Some(new_ctx_idx);
            return Vec::default();
        }

        // If we are matching a specific context then use it.
        let mut highlights = Vec::default();
        if let Some(cur_ctx_idx) = self.cur_ctx {
            let ctx_matcher = &self.config.ctx_matches[cur_ctx_idx];

            // Firstly check if we're matching this context's exit pattern.
            if ctx_matcher
                .ctx_exit_re
                .iter()
                .any(|exit_re| exit_re.is_match(next_line))
            {
                // Set the current context to none.
                self.cur_ctx = None;
            } else {
                // Find any matches for this context.
                for HighlightMatch { re, colour } in &ctx_matcher.matches {
                    if let Some(caps) = re.captures(next_line) {
                        let mtch = caps
                            .get(if caps.len() == 1 { 0 } else { 1 })
                            .expect("BUG! `caps` is guaranteed to have at least one match.");

                        highlights.push(Highlight {
                            begin: mtch.start(),
                            end: mtch.end(),
                            colour: *colour,
                        });
                    }
                }
            }
        }

        highlights
    }
}
