use std::{
    io::{self, BufRead, BufReader, Read, Write},
    os::unix::fs::PermissionsExt,
};

mod highlight;
mod iface;
mod mbox;

// -------------------------------------------------------------------------------------------------

fn main() {
    std::process::exit(smbox().map(|_| 0).unwrap_or_else(|err| {
        println!("{}", err);
        1
    }));
}

// -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -

fn smbox() -> io::Result<()> {
    let mbox_file = std::fs::File::open(mbox::get_mbox_path()?)?;
    let lines = BufReader::new(mbox_file)
        .lines()
        .collect::<io::Result<Vec<_>>>()?;

    if lines.is_empty() {
        println!("No mail.");
    } else {
        let config = read_config().unwrap_or_default();
        let messages = mbox::Mbox::from_lines(lines);
        let mut highlighter = highlight::load_highlighter(&config)
            .map_err(|s| io::Error::new(io::ErrorKind::InvalidData, s))?;
        let actions = iface::run(&messages, &mut highlighter)?;

        if !actions.is_empty() {
            println!(
                "{}",
                match perform_actions(&messages, actions)? {
                    n if n == messages.count() as i64 => "Deleted all messages.".to_owned(),
                    1 => "Deleted message.".to_owned(),
                    n => format!("Deleted {n} messages."),
                }
            );
        }
    }
    Ok(())
}

// -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -

fn read_config() -> io::Result<String> {
    let base_dirs = directories::BaseDirs::new().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Failed to determine config file path.",
        )
    })?;
    let mut config_file_path = base_dirs.config_dir().to_owned();
    config_file_path.push("smbox.toml");

    let mut config = String::new();
    io::BufReader::new(std::fs::File::open(config_file_path)?).read_to_string(&mut config)?;
    Ok(config)
}

// -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -

fn perform_actions(mbox: &mbox::Mbox, actions: Vec<iface::Action>) -> io::Result<i64> {
    // Right now we only support DeleteMessage actions.  We should get a compile error about
    // non-exhaustive pattern matches if/when there are other actions introducted in the future.
    assert!(actions.iter().all(|action| match action {
        iface::Action::DeleteMessage(_) => true,
    }));

    let message_is_deleted = |idx: usize| {
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
            if actions.len() < mbox.count() {
                // Write the messages we're keeping.
                for msg_idx in 0..mbox.count() {
                    if !message_is_deleted(msg_idx) {
                        for line in mbox.all_lines(msg_idx).unwrap_or(&[]).iter() {
                            // I'm not 100% happy with this.  Perhaps writing the line with
                            // std::fs::File::write() and then writing the newline separately would
                            // be better?
                            std::writeln!(temp_mbox_file, "{line}")?;
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
