use std::{
    fs,
    io::{self, BufRead, BufReader, Write},
    iter::FromIterator,
    os::unix::fs::PermissionsExt,
};

use anyhow::Context;

mod highlight;
mod iface;
mod mbox;

// -------------------------------------------------------------------------------------------------

fn main() -> anyhow::Result<()> {
    let mbox_path = mbox::get_mbox_path()?;
    let mbox_file = fs::File::open(&mbox_path)
        .with_context(|| format!("Failed to open mbox file '{mbox_path}'."))?;

    let lines = BufReader::new(mbox_file)
        .lines()
        .collect::<Result<Vec<_>, _>>()?;

    if lines.is_empty() {
        println!("No mail.");
    } else {
        let config = read_config_str().unwrap_or_default();
        let messages = mbox::Mbox::from_iter(lines);

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

fn read_config_str() -> anyhow::Result<String> {
    let base_dirs =
        directories::BaseDirs::new().context("Failed to determine config file path.")?;

    let mut config_file_path = base_dirs.config_dir().to_owned();
    config_file_path.push("smbox.toml");

    Ok(fs::read_to_string(config_file_path)?)
}

// -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -  -

fn write_mbox(mbox: &mbox::Mbox) -> anyhow::Result<i64> {
    // Create a replacement mbox file with remaining messages.
    let mut num_deleted_messages = 0;
    {
        let temp_mbox_file_path = mktemp::Temp::new_file()?;
        {
            let mut temp_mbox_file = fs::OpenOptions::new()
                .write(true)
                .open(&temp_mbox_file_path)?;

            // Make sure the permissions are rw------- even though that seems to be the default.
            temp_mbox_file.metadata()?.permissions().set_mode(0o600);

            // Write the messages we're keeping.
            for msg in mbox.iter() {
                if !msg.has_status(mbox::Status::Deleted) {
                    for line in msg.all_lines().iter() {
                        writeln!(temp_mbox_file, "{line}")?;
                    }
                } else {
                    num_deleted_messages += 1;
                }
            }
        }

        fs::copy(temp_mbox_file_path, mbox::get_mbox_path()?)?;
    }

    Ok(num_deleted_messages)
}

// -------------------------------------------------------------------------------------------------
