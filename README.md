# smbox

A minimalist mbox reader.

## About

**smbox** is for people (like me) who may have remote servers which have cron jobs or other system
services reporting via local system mail.

Using [Mutt](http://www.mutt.org/) or a similarly fully fledged terminal based client to read system mail feels like
overkill but using plain old [MAIL(1)](https://linux.die.net/man/1/mail) is just a bit too old school, and so **smbox** tries to sit in
between.

## Current Features

  * Can read from `$MAIL` and display messages using a basic TUI.
  * Can delete messages.
  * Can highlight sections of the email bodies using regular expressions.

## Caveats
  * Barely tested though I use it every day.
  * It's designed to work with unix OSes and uses some unix specific library calls.  I haven't tried
    it on MacOS and I can't imagine how it would even work on Windows if you did force it to
    compile.

## Config File

The config file is found at `${CONFIG_DIR}/smbox.ron`, e.g., `~/.config/smbox.ron`.  It is
[RON](https://github.com/ron-rs/ron) and can contain the declarations needed for highlighting parts of the email message text.

It has the following format:

```
(
    highlights: [
        (   // First highlighter.
            enter: <regex>,
            exit: Some(<regex>),
            matchers:[
                (match: <regex 1>, colour: <clr 1>),
                (match: <regex 2>, colour: <clr 2>),
                .
                .
                (match: <regex n>, colour: <clr n>),
            ]
        ),
        (   // Second highlighter...
            enter: <regex>,
            matchers:[
                (match: <regex 1>, colour: <clr 1>),
                (match: <regex 2>, colour: <clr 2>),
                .
                .
                (match: <regex n>, colour: <clr n>),
            ]
        ),
    ]
)
etc.
```

* `regex` is a string which will work with the `[regex](https://github.com/rust-lang/regex)` crate.
* The `enter` regular expression value turns that context on.
* The `exit` regular expression turns it off again, and is optional (but must be `Some(..)` when present).
* The `matchers` are pairs of regular expressions and 256 colour indices which will be matched
  against lines of text while that context is 'alive'.
* The regular expressions may contain captures (between parentheses) in which case the first
  capture will be highlighted rather than the entire match, as is the default.

E.g.,
```
(
    highlights: [
        (   // Login failures.  Enter this context when we see 'login failures:'.  Exit on a blank line.
            enter: " login failures:$",
            exit: Some("^$"),
            matchers:[
                // Highlight reported unknown users in colour 140.
                (match: "unknown user (.*) ", colour: 140),
                // Highlight the words 'invalid protocol' when found, with colour 87.
                (match: "invalid protocol", colour: 87),
            ]
        ),
        (   // Let's encrypt.
            enter: "^Processing.*letsencrypt",
            matchers: [
                (match: "^Certificate not yet due for renewal$",     colour: 107),
                (match: "^Renewing an existing certificate for .*$", colour: 222),
                (match: "^Congratulations, all renewals succeeded:", colour: 107),
            ]
        ),
    ]
)
```
