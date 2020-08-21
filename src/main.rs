use std::io::BufRead;

mod mbox;
mod iface;

// -------------------------------------------------------------------------------------------------
// Keys:
//   Selector:
//     enter   - view in pager
//     j/k     - down/up
//     d/u     - delete/undelete
//     q       - write changes, exit
//     x       - discard changes, exit
//   Pager:
//     space/f - page down
//     b       - page up
//     g/G     - goto top/bottom
//     d/u     - delete/undelete
//     q       - keep changes, return
//     x       - discard changes, return
//
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
        let actions = iface::run(&lines, &messages);

        for action in actions {
            println!("{:?}", action)
        }
    }

    println!("");
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

// -------------------------------------------------------------------------------------------------


// -------------------------------------------------------------------------------------------------
