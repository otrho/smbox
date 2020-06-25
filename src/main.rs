use std::io::BufRead;

// -------------------------------------------------------------------------------------------------
// [X] Find mbox.
// [ ] Read all lines.
// [ ] Divide into messages:
//     [ ] '^From' divider.
//     [ ] Useful headers, especially 'Subject:'.
//     [ ] Hidden headers.
//     [ ] Body.
// [ ] UI:
//     [ ] Message selector, headers summary.
//     [ ] Show messages (via pager?)
//     [ ] Colours.
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

fn smbox() -> std::io::Result<()> {
    // Read lines as a vector of strings from the mbox path found in $MAIL.
    let lines = read_lines(get_mbox_path()?)?;
    if lines.is_empty() {
        println!("No mail.");
    }

    Ok(())
}

// -------------------------------------------------------------------------------------------------

fn read_lines(path: String) -> std::io::Result<Vec<String>> {
    let reader = std::io::BufReader::new(std::fs::File::open(path)?);
    let mut lines = Vec::<String>::new();
    for line in reader.lines() {
        lines.push(line?);
    }
    Ok(lines)
}

// -------------------------------------------------------------------------------------------------

fn get_mbox_path() -> std::io::Result<String> {
    match std::env::vars_os().find(|(key, _)| key == "MAIL") {
        None => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "error: Unable to determine mbox path; missing MAIL environment variable.",
        )),
        Some((_, env_value)) => env_value.into_string().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "error: Malformed string in MAIL environment variable.",
            )
        }),
    }
}

// -------------------------------------------------------------------------------------------------
