#[allow(dead_code)]
mod app;

#[allow(clippy::module_inception)]
mod ui;

use app::App;
use clarinet_deployments::onchain::{DeploymentEvent, TransactionTracker};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::sync::mpsc::Receiver;

pub fn start_ui(
    node_url: &str,
    deployment_event_rx: Receiver<DeploymentEvent>,
    transaction_trackers: Vec<TransactionTracker>,
) -> Result<(), String> {
    enable_raw_mode().expect("unable to setup user interface");

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen).expect("unable to setup user interface");

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("unable to setup user interface");
    let mut app = App::new(node_url, transaction_trackers);

    let res = loop {
        terminal
            .draw(|f| ui::draw(f, &mut app))
            .expect("unable to setup user interface");
        match deployment_event_rx.recv() {
            Ok(DeploymentEvent::TransactionUpdate(update)) => {
                app.display_contract_status_update(update);
            }
            Ok(DeploymentEvent::DeploymentCompleted) => {
                break Ok(());
            }
            Ok(DeploymentEvent::Interrupted(message)) => {
                break Err(message);
            }
            Err(e) => break Err(format!("{e:?}")),
        }
    };
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen,);
    let _ = terminal.show_cursor();
    res
}
