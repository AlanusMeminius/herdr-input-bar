mod bar;
mod editor;
mod herdr;
mod open;
mod state;

use std::env;
use std::process;

fn main() {
    let status = match env::args().nth(1).as_deref() {
        Some("open") => open::run(),
        Some("bar") => bar::run(),
        Some(cmd) => {
            eprintln!("unknown command: {cmd}");
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "usage: herdr-input-bar open|bar",
            ))
        }
        None => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "usage: herdr-input-bar open|bar",
        )),
    };

    if let Err(e) = status {
        eprintln!("herdr-input-bar: {e}");
        process::exit(1);
    }
}
