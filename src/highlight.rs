// -------------------------------------------------------------------------------------------------
// Hooley dooley, this TOML parsing code is super fragile, and really only a proof of concept at
// this stage.  Needs to be made much more robust.
//
// The TOML library itself has try_into() and functions like as_table() which are a little better,
// and then instead maybe we should use serde style parsing.

pub fn load_highlighter(toml_config: &str) -> Result<Highlighter, String> {
    let toml_value = toml_config
        .parse::<toml::Value>()
        .map_err(|err| format!("{}", err))?;

    let val_to_regex = |val: &toml::value::Value| match val {
        toml::value::Value::String(s) => {
            regex::Regex::new(s).unwrap_or_else(|_| panic!("Invalid regex: {}", s))
        }
        _ => panic!("Expecting string regex value'."),
    };

    let val_to_matchers = |val: &toml::value::Value| match val {
        toml::value::Value::Array(matchers) => matchers
            .iter()
            .map(|pair| match pair {
                toml::value::Value::Array(pair_ary) => (
                    val_to_regex(&pair_ary[0]),
                    pair_ary[1]
                        .as_integer()
                        .expect("Expecting integer for colour index.") as u8,
                ),
                _ => panic!("Expecting array for matcher regex and colour pair."),
            })
            .collect(),
        _ => panic!("Expecting pair of regex and colour index for matcher."),
    };

    match toml_value {
        toml::value::Value::Table(root_table) => {
            let mut ctx_matchers = Vec::new();
            for (ctx_id, val) in root_table {
                if let toml::value::Value::Table(section) = val {
                    let ctx_enter_re = section
                        .get("re")
                        .map(val_to_regex)
                        .unwrap_or_else(|| panic!("Missing 're' value for '{}' context.", ctx_id));

                    let ctx_exit_re = section.get("exit").map(val_to_regex);

                    let matchers =
                        section
                            .get("matchers")
                            .map(val_to_matchers)
                            .unwrap_or_else(|| {
                                panic!("Missing 'matchers' value for '{}' context.", ctx_id)
                            });

                    ctx_matchers.push(HighlightContext {
                        ctx_enter_re,
                        ctx_exit_re,
                        matchers,
                    });
                }
            }

            Ok(Highlighter {
                ctx_matchers,
                cur_ctx: None,
            })
        }
        _ => Err("BUG! Config is not a table?".to_owned()),
    }
}

// -------------------------------------------------------------------------------------------------

type Colour256 = u8;

pub struct Highlighter {
    ctx_matchers: Vec<HighlightContext>,
    cur_ctx: Option<usize>,
}

struct HighlightContext {
    ctx_enter_re: regex::Regex,
    ctx_exit_re: Option<regex::Regex>,
    matchers: Vec<(regex::Regex, Colour256)>,
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
