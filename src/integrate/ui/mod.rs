#[allow(dead_code)]
mod app;
#[allow(dead_code)]
mod ui;
#[allow(dead_code)]
mod util;

use super::{events_observer::EventsObserverCommand, DevnetEvent};
use app::App;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::sync::mpsc::{Receiver, Sender};
use std::{
    error::Error,
    io::{stdout, Stdout},
    thread,
    time::{Duration, Instant},
};
use crate::types::StacksChainEvent;
use tui::{backend::CrosstermBackend, Terminal};

pub fn start_ui(
    devnet_events_tx: Sender<DevnetEvent>,
    devnet_events_rx: Receiver<DevnetEvent>,
    events_observer_commands_tx: Sender<EventsObserverCommand>,
    orchestrator_terminator_tx: Sender<bool>,
    orchestrator_terminated_rx: Receiver<bool>,
    devnet_path: &str,
) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;

    // Setup input handling
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
                if let Err(_) = devnet_events_tx.send(DevnetEvent::Tick) {
                    break;
                }
                last_tick = Instant::now();
            }
        }
    });

    let mut app = App::new("Clarinet", devnet_path);
    terminal.clear()?;

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;
        match devnet_events_rx.recv()? {

            DevnetEvent::KeyEvent(event) => match (event.modifiers, event.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                    app.display_log(DevnetEvent::log_warning("Ctrl+C received, initiating termination sequence.".into()));
                    let _ = trigger_reset(
                        true,
                        &events_observer_commands_tx,
                        &orchestrator_terminator_tx);

                    let _ = terminate(
                        &mut terminal,
                        orchestrator_terminated_rx);
                    break;
                }
                (_, KeyCode::Char('0')) => {
                    // Reset Testnet`
                    app.reset();
                    app.display_log(DevnetEvent::log_warning("Reset Devnet...".into()));
                    let _ = trigger_reset(
                        false,
                        &events_observer_commands_tx,
                        &orchestrator_terminator_tx);
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
            DevnetEvent::ServiceStatus(status) => {
                app.display_service_status_update(status);
            }
            DevnetEvent::StacksChainEvent(chain_event) => {
                if let StacksChainEvent::ChainUpdatedWithBlock(block) = chain_event {
                    app.display_block(block);
                } else {
                    // todo(lgalabru)
                }
            }
            DevnetEvent::BitcoinChainEvent(_chain_event) => {
            }
            DevnetEvent::MempoolAdmission(tx) => {
                app.update_mempool(tx);
            }
            // DevnetEvent::Terminate => {

            // },
            // DevnetEvent::Restart => {

            // },
        }
        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn trigger_reset(
    terminate: bool,
    events_observer_commands_tx: &Sender<EventsObserverCommand>,
    orchestrator_terminator_tx: &Sender<bool>,
) -> Result<(), Box<dyn Error>> {
    events_observer_commands_tx
        .send(EventsObserverCommand::Terminate(true))
        .expect("Unable to terminate devnet");
    orchestrator_terminator_tx
        .send(terminate)
        .expect("Unable to terminate devnet");
    Ok(())
}

fn terminate(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    orchestrator_terminated_rx: Receiver<bool>,
) -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    match orchestrator_terminated_rx.recv()? {
        _ => {}
    }
    terminal.show_cursor()?;
    Ok(())
}
