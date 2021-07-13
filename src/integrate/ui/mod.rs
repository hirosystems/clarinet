// Mockup
// Section 1 (10 lines)
// | Devnet orchestration (75%)      | Endpoints (25%)                 |
// | Gauge (boot, yellow or green)   | Table (Name | Address | Status) |
// | Logs                            |                                 |
// Section 2 (5 lines)
// | Blocks Tab + PoX Tab (100%)               |
// Section 3 (Else)
// | Details about block, Transactions, Asset map.
// Section 4
// | Mempool state

#[allow(dead_code)]
mod app;
#[allow(dead_code)]
mod ui;
#[allow(dead_code)]
mod util;

use std::sync::mpsc::{Sender, Receiver};
use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    error::Error,
    io::{stdout, Stdout},
    thread,
    time::{Duration, Instant},
};
use tui::{backend::CrosstermBackend, Terminal};
use super::{DevnetEvent, LogData, LogLevel};

pub fn start_ui(devnet_events_tx: Sender<DevnetEvent>, devnet_events_rx: Receiver<DevnetEvent>, events_observer_terminator_tx: Sender<bool>, orchestrator_terminator_tx: Sender<bool>, orchestrator_terminated_rx: Receiver<bool>) -> Result<(), Box<dyn Error>> {

    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;

    // Setup input handling
    let enhanced_graphics = false;
    let tick_rate = Duration::from_millis(500);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            // poll for tick rate duration, if no events, sent tick event.
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            if event::poll(timeout).unwrap() {
                if let Event::Key(key) = event::read().unwrap() {
                    devnet_events_tx.send(DevnetEvent::KeyEvent(key)).unwrap();
                }
            }
            if last_tick.elapsed() >= tick_rate {
                devnet_events_tx.send(DevnetEvent::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });

    let mut app = App::new("Clarinet", enhanced_graphics);
    terminal.clear()?;

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;
        match devnet_events_rx.recv()? {

            DevnetEvent::KeyEvent(event) => match (event.modifiers, event.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                    app.display_log(DevnetEvent::log_warning("Ctrl+C received, initiating termination sequence.".into()));
                    terminate(
                        &mut terminal, 
                        true, 
                        events_observer_terminator_tx, 
                        orchestrator_terminator_tx, 
                        orchestrator_terminated_rx);
                    break;
                }
                (_, KeyCode::Char('0')) => {
                    // Reset Testnet
                },
                (_, KeyCode::Left) => app.on_left(),
                (_, KeyCode::Up) => app.on_up(),
                (_, KeyCode::Right) => app.on_right(),
                (_, KeyCode::Down) => app.on_down(),
                _ => {}
            },
            DevnetEvent::Tick => {
                app.on_tick();
            },
            DevnetEvent::Log(log) => {
                app.display_log(log);
            },
            DevnetEvent::Terminate => {

            },
            DevnetEvent::Restart => {

            },
            DevnetEvent::ServiceStatus(status) => {
                app.display_service_status_update(status);
            }
            DevnetEvent::Block(block) => {
                app.display_block(block);
            }
            DevnetEvent::Microblock(microblock) => {

            }
            DevnetEvent::MempoolAdmission(tx) => {

            }
        }
        if app.should_quit {
            break;
        }
    }

    Ok(())
}


fn terminate(terminal: &mut Terminal<CrosstermBackend<Stdout>>, terminate: bool, events_observer_terminator_tx: Sender<bool>, orchestrator_terminator_tx: Sender<bool>, orchestrator_terminated_rx: Receiver<bool>) -> Result<(), Box<dyn Error>>  {

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    orchestrator_terminator_tx.send(terminate)
        .expect("Unable to terminate devnet");
    events_observer_terminator_tx.send(terminate)
        .expect("Unable to terminate devnet");
    
    match orchestrator_terminated_rx.recv()? {
        _ => {}
    }
    terminal.show_cursor()?;

    Ok(())
}