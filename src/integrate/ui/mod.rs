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
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    error::Error,
    io::stdout,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use tui::{backend::CrosstermBackend, Terminal};
use super::DevnetEvent;

pub fn start_ui(event_tx: Sender<DevnetEvent>, event_rx: Receiver<DevnetEvent>) -> Result<(), Box<dyn Error>> {

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
                if let CEvent::Key(key) = event::read().unwrap() {
                    event_tx.send(DevnetEvent::KeyEvent(key)).unwrap();
                }
            }
            if last_tick.elapsed() >= tick_rate {
                event_tx.send(DevnetEvent::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });

    let mut app = App::new("Clarinet", enhanced_graphics);
    terminal.clear()?;

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;
        match event_rx.recv()? {
            DevnetEvent::KeyEvent(event) => match event.code {
                KeyCode::Char('q') => {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    terminal.show_cursor()?;
                    break;
                }
                KeyCode::Char('0') => {
                },
                KeyCode::Char(c) => app.on_key(c),
                KeyCode::Left => app.on_left(),
                KeyCode::Up => app.on_up(),
                KeyCode::Right => app.on_right(),
                KeyCode::Down => app.on_down(),
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
