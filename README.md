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
  * The config file parsing is extremely fragile and needs to be toughened up considerably.

## Config File

The config file is found at `${CONFIG_DIR}/smbox.toml`, e.g., `~/.config/smbox.toml`.  It is TOML
and can contain the declarations needed for highlighting parts of the email message text.

It has the following format:

```
exit_re = <regex>

[<context-name 0>]
re = <regex>
matchers = [
  [<regex 1>, <clr 1>],
  [<regex 2>, <clr 2>],
  .
  .
  [<regex n>, <clr n>],
]

[<context-name 1>]
re = <regex>
matchers = [
  [<regex 1>, <clr 1>],
  [<regex 2>, <clr 2>],
  .
  .
  [<regex n>, <clr n>],
]

etc.
```

* `<context-name n>` is the name of a context where the following regular expressions are 'alive'.
* The `re` value turns that context on.
* And the global `exit_re` will reset the contexts to off.
* The `matchers` are pairs of regular expressions and 256 colour indices which will be matched
  against lines of text while that context is 'alive'.
* The regular expressions may contain captures (between parentheses) in which case the first
  capture will be highlighted rather than the entire match, as is the default.

E.g.,
```
# Reset contexts on empty lines.
exit_re = '^$'

[login-failures]
re = ' login failures:$'
matchers = [
  # Highlight reported unknown users in colour 140.
  ['unknown user (.*) ', 140],
  # Highlight the words 'invalid protocol' when found with colour 87.
  ['invalid protocol', 87],
]
```
