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
    let exit_code = match smbox() {
        Ok(_) => 0,
        Err(err) => {
            println!("{}", err);
            1
        }
    };
    std::process::exit(exit_code)
}

fn smbox() -> Result<(), std::io::Error> {
    let mail_path = get_mbox_path()?;
    println!("mbox is at '{}'", mail_path);
    Ok(())
}

// -------------------------------------------------------------------------------------------------

fn get_mbox_path() -> Result<String, std::io::Error> {
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
