use std::io::BufRead;

mod iface;
mod mbox;

// -------------------------------------------------------------------------------------------------

fn main() {
    std::process::exit(match smbox() {
        Ok(_) => 0,
        Err(err) => {
            println!("{}", err);
            1
        }
    });
}

// -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -

fn smbox() -> std::io::Result<()> {
    // Read lines as a vector of strings from the mbox path found in $MAIL.
    let lines = read_lines(mbox::get_mbox_path()?)?;
    if lines.is_empty() {
        println!("No mail.");
    } else {
        let messages = mbox::parse_mbox(&lines);
        let actions = iface::run(&lines, &messages)?;

        if !actions.is_empty() {
            let num_deleted_messages = perform_actions(&lines, &messages, actions)?;
            if num_deleted_messages == messages.len() as i64 {
                println!("Deleted all messages.");
            } else if num_deleted_messages > 0 {
                println!("Deleted {} message(s).", num_deleted_messages);
            }
        }
    }
    Ok(())
}

// -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -

fn read_lines(path: String) -> std::io::Result<Vec<String>> {
    let reader = std::io::BufReader::new(std::fs::File::open(path)?);
    let mut lines = Vec::<String>::new();
    for line in reader.lines() {
        lines.push(line?);
    }
    Ok(lines)
}

// -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -

use std::io::Write;
use std::os::unix::fs::PermissionsExt;

fn perform_actions(
    lines: &Vec<String>,
    messages: &Vec<mbox::Message>,
    actions: Vec<iface::Action>,
) -> std::io::Result<i64> {
    // Right now we only support DeleteMessage actions.  We should get a compile error about
    // non-exhaustive pattern matches if/when there are other actions introducted in the future.
    assert!(actions.iter().all(|action| match action {
        iface::Action::DeleteMessage(_) => true,
    }));

    let message_is_deleted = |idx: i64| {
        actions.iter().any(|action| match action {
            iface::Action::DeleteMessage(del_idx) => idx == *del_idx,
        })
    };

    // Create a replacement mbox file with remaining messages.
    {
        let temp_mbox_file_path = mktemp::Temp::new_file()?;
        {
            let mut temp_mbox_file = std::fs::OpenOptions::new()
                .write(true)
                .open(&temp_mbox_file_path)?;

            // Make sure the permissions are rw------- even though that seems to be the default.
            temp_mbox_file.metadata()?.permissions().set_mode(0o600);

            // If the number of deletions is the number of messages then we are deleting all messages and
            // don't need to write to the new mbox file at all.
            if actions.len() < messages.len() {
                // Write the messages we're keeping.
                for (idx, msg) in messages.iter().enumerate() {
                    if !message_is_deleted(idx as i64) {
                        for line_idx in msg.start_idx..msg.end_idx {
                            // I'm not 100% happy with this.  Perhaps writing the line with
                            // std::fs::File::write() and then writing the newline separately would
                            // be better?
                            std::writeln!(temp_mbox_file, "{}", lines[line_idx as usize])?;
                        }
                    }
                }
            }
        }

        std::fs::copy(temp_mbox_file_path, mbox::get_mbox_path()?)?;
    }

    // While we only have a delete action and no others we can assume number of deleted messages is
    // the number of actions.
    let num_deleted_msgs = actions.len() as i64;
    Ok(num_deleted_msgs)
}

// -------------------------------------------------------------------------------------------------
