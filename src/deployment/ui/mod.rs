#[allow(dead_code)]
mod app;
#[allow(dead_code)]
mod ui;

use super::PublishUpdate;
use app::App;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::sync::mpsc::Receiver;
use std::{error::Error, io::stdout};
use tui::{backend::CrosstermBackend, Terminal};

pub fn start_ui(
    node_url: &str,
    per_contract_event_rx: Receiver<PublishUpdate>,
    contracts: Vec<(String, String)>,
) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new(node_url, contracts);

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;
        match per_contract_event_rx.recv()? {
            PublishUpdate::ContractUpdate(update) => {
                app.display_contract_status_update(update);
            }
            PublishUpdate::Completed => {
                disable_raw_mode()?;
                execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
                terminal.show_cursor()?;
                break;
            }
        }
    }

    Ok(())
}
