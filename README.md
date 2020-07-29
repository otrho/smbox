# smbox

A minimalist mbox reader.

## About

**smbox** is for people (like me) who may have remote servers which have cron jobs or other system
services reporting via local system mail.

Using [Mutt](http://www.mutt.org/) or a similar fully fledged terminal based client to read system
mail feels like overkill but using plain old [MAIL(1)](https://linux.die.net/man/1/mail) is just a
bit too old school, and so **smbox** tries to sit in between.

**Currently smbox is very much a work in progress.**

## Current Features

  * Can read from `$MAIL` and display messages using a _very_ basic TUI.
  * Is written in Rust..!

## Coming Features
  * A better TUI with a nice summary list and colours.
  * The ability to delete messages.
  * Keyword matching, useful for highlighting particular parts of email bodies, especially errors or
    problems reported by the system.

