mod app;

use crate::app::run;
use color_eyre::Result;
use crossterm::cursor::{Hide, Show};
use crossterm::ExecutableCommand;
use std::io::stdout;

fn main() -> Result<()> {
    setup_terminal()?;

    run()?;

    teardown_terminal();
    Ok(())
}

pub fn setup_terminal() -> Result<()> {
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        teardown_terminal();
        default_panic(info);
    }));

    crossterm::terminal::enable_raw_mode()?;
    stdout().execute(Hide)?;

    Ok(())
}

pub fn teardown_terminal() {
    let result = crossterm::terminal::disable_raw_mode();
    if let Err(err) = result {
        eprintln!(
            "Failed to disable raw mode, you'll need to close this terminal window:\n{}",
            err
        );
    }
    let mut stdout = stdout();
    let result = stdout.execute(Show);
    if let Err(err) = result {
        eprintln!(
            "Failed to restore cursor, you'll need to close this terminal window:\n{}",
            err
        );
    }
    println!("\n\n");
}
