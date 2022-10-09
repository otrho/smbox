use std::{
    io::{self, BufRead, BufReader, Read, Write},
    iter::FromIterator,
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
        let messages = mbox::Mbox::from_iter(lines.into_iter());

        let mut highlighter = highlight::load_highlighter(&config)
            .map_err(|s| io::Error::new(io::ErrorKind::InvalidData, s))?;

        if let Some(mut updated_messages) = iface::run(messages, &mut highlighter)? {
            for msg in updated_messages.iter_mut() {
                msg.set_status(mbox::Status::NonRecent);
            }

            println!(
                "{}",
                match write_mbox(&updated_messages)? {
                    n if n == updated_messages.count() as i64 => "Deleted all messages.".to_owned(),
                    1 => "Deleted 1 message.".to_owned(),
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

fn write_mbox(mbox: &mbox::Mbox) -> io::Result<i64> {
    // Create a replacement mbox file with remaining messages.
    let mut num_deleted_messages = 0;
    {
        let temp_mbox_file_path = mktemp::Temp::new_file()?;
        {
            let mut temp_mbox_file = std::fs::OpenOptions::new()
                .write(true)
                .open(&temp_mbox_file_path)?;

            // Make sure the permissions are rw------- even though that seems to be the default.
            temp_mbox_file.metadata()?.permissions().set_mode(0o600);

            // Write the messages we're keeping.
            for msg in mbox.iter() {
                if !msg.has_status(mbox::Status::Deleted) {
                    for line in msg.all_lines().iter() {
                        std::writeln!(temp_mbox_file, "{line}")?;
                    }
                } else {
                    num_deleted_messages += 1;
                }
            }
        }

        std::fs::copy(temp_mbox_file_path, mbox::get_mbox_path()?)?;
    }

    Ok(num_deleted_messages)
}

// -------------------------------------------------------------------------------------------------
