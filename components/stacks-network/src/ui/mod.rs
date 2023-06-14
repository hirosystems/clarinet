#[allow(dead_code)]
mod app;
#[allow(dead_code)]
mod ui;
#[allow(dead_code)]
mod util;

use super::DevnetEvent;
use crate::{chains_coordinator::BitcoinMiningCommand, ChainsCoordinatorCommand};
use app::App;
use chainhook_sdk::chainhook_types::StacksChainEvent;
use chainhook_sdk::utils::Context;
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
use tui::{backend::CrosstermBackend, Terminal};

pub fn start_ui(
    devnet_events_tx: Sender<DevnetEvent>,
    devnet_events_rx: Receiver<DevnetEvent>,
    chains_coordinator_commands_tx: crossbeam_channel::Sender<ChainsCoordinatorCommand>,
    orchestrator_terminated_rx: Receiver<bool>,
    devnet_path: &str,
    subnet_enabled: bool,
    automining_enabled: bool,
    ctx: &Context,
) -> Result<(), String> {
    let res = do_start_ui(
        devnet_events_tx,
        devnet_events_rx,
        chains_coordinator_commands_tx,
        orchestrator_terminated_rx,
        devnet_path,
        subnet_enabled,
        automining_enabled,
        ctx,
    );
    if let Err(ref _e) = res {
        // potential additional cleaning
    }
    res
}

pub fn do_start_ui(
    devnet_events_tx: Sender<DevnetEvent>,
    devnet_events_rx: Receiver<DevnetEvent>,
    chains_coordinator_commands_tx: crossbeam_channel::Sender<ChainsCoordinatorCommand>,
    orchestrator_terminated_rx: Receiver<bool>,
    devnet_path: &str,
    subnet_enabled: bool,
    automining_enabled: bool,
    ctx: &Context,
) -> Result<(), String> {
    enable_raw_mode().map_err(|e| format!("unable to start terminal ui: {}", e.to_string()))?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)
        .map_err(|e| format!("unable to start terminal ui: {}", e.to_string()))?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)
        .map_err(|e| format!("unable to start terminal ui: {}", e.to_string()))?;

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

    let mut app = App::new("Clarinet", devnet_path, subnet_enabled);

    terminal
        .clear()
        .map_err(|e| format!("unable to start terminal ui: {}", e.to_string()))?;

    let mut mining_command_tx: Option<Sender<BitcoinMiningCommand>> = None;

    loop {
        terminal
            .draw(|f| ui::draw(f, &mut app))
            .map_err(|e| format!("unable to update ui: {}", e.to_string()))?;
        let event = match devnet_events_rx.recv() {
            Ok(event) => event,
            Err(_e) => {
                let _ = terminate(
                    &mut terminal,
                    chains_coordinator_commands_tx,
                    orchestrator_terminated_rx,
                );
                break;
            }
        };
        match event {
            DevnetEvent::KeyEvent(event) => match (event.modifiers, event.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                    app.display_log(DevnetEvent::log_warning("Ctrl+C received, initiating termination sequence.".into()), ctx);
                    let _ = terminate(
                        &mut terminal,
                        chains_coordinator_commands_tx,
                        orchestrator_terminated_rx,
                        );
                    break;
                }
                (KeyModifiers::NONE, KeyCode::Char('n')) => {
                    if let Some(ref tx) = mining_command_tx {
                        let _ = tx.send(BitcoinMiningCommand::Mine);
                        app.display_log(DevnetEvent::log_success(format!("Bitcoin block mining triggered manually")), ctx);
                    } else {
                        app.display_log(DevnetEvent::log_error(format!("Manual block mining not ready")), ctx);
                    }
                }
                (KeyModifiers::NONE, KeyCode::Left) => app.on_left(),
                (KeyModifiers::NONE, KeyCode::Up) => app.on_up(),
                (KeyModifiers::NONE, KeyCode::Right) => app.on_right(),
                (KeyModifiers::NONE, KeyCode::Down) => app.on_down(),
                _ => {}
            },
            DevnetEvent::Tick => {
                app.on_tick();
            },
            DevnetEvent::Log(log) => {
                app.display_log(log, ctx);
            },
            DevnetEvent::ServiceStatus(status) => {
                app.display_service_status_update(status);
            }
            DevnetEvent::StacksChainEvent(chain_event) => {
                match chain_event {
                    StacksChainEvent::ChainUpdatedWithBlocks(update) => {
                        let raw_txs = if app.mempool.items.is_empty() {
                            vec![]
                        } else {
                            update.new_blocks.iter().flat_map(|b| b.block.transactions.iter().map(|tx| tx.metadata.raw_tx.as_str())).collect::<Vec<_>>()
                        };

                        let mut indices_to_remove = vec![];
                        for (idx, item) in app.mempool.items.iter().enumerate() {
                            if raw_txs.contains(&item.tx_data.as_str()) {
                                indices_to_remove.push(idx);
                            }
                        }

                        indices_to_remove.reverse();
                        for i in indices_to_remove {
                            app.mempool.items.remove(i);
                        }
                        for block_update in update.new_blocks.into_iter() {
                            app.display_block(block_update.block);
                        }
                    }
                    StacksChainEvent::ChainUpdatedWithMicroblocks(update) => {
                        let raw_txs = if app.mempool.items.is_empty() {
                            vec![]
                        } else {
                            update.new_microblocks.iter().flat_map(|b| b.transactions.iter().map(|tx| tx.metadata.raw_tx.as_str())).collect::<Vec<_>>()
                        };

                        let mut indices_to_remove = vec![];
                        for (idx, item) in app.mempool.items.iter().enumerate() {
                            if raw_txs.contains(&item.tx_data.as_str()) {
                                indices_to_remove.push(idx);
                            }
                        }

                        indices_to_remove.reverse();
                        for i in indices_to_remove {
                            app.mempool.items.remove(i);
                        }
                        for block_update in update.new_microblocks.into_iter() {
                            app.display_microblock(block_update);
                        }
                    }
                    _ => {} // handle display on re-org, theorically unreachable in context of devnet
                }
            }
            DevnetEvent::BitcoinChainEvent(_chain_event) => {
            }
            DevnetEvent::MempoolAdmission(tx) => {
                app.add_to_mempool(tx);
            }
            DevnetEvent::ProtocolDeployingProgress(_) => {
                // Display something
            }
            DevnetEvent::FatalError(message) => {
                app.display_log(DevnetEvent::log_error(format!("Fatal: {}", message)), ctx);
                let _ = terminate(
                    &mut terminal,
                    chains_coordinator_commands_tx,
                    orchestrator_terminated_rx,
                    );
                return Err(message)
            },
            DevnetEvent::BootCompleted(bitcoin_mining_tx) => {
                app.display_log(DevnetEvent::log_success("Local Devnet network ready".into()), ctx);
                if automining_enabled {
                    let _ = bitcoin_mining_tx.send(BitcoinMiningCommand::Start);
                }
                mining_command_tx = Some(bitcoin_mining_tx);
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

fn terminate(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    chains_coordinator_commands_tx: crossbeam_channel::Sender<ChainsCoordinatorCommand>,
    orchestrator_terminated_rx: Receiver<bool>,
) -> Result<(), Box<dyn Error>> {
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let res = chains_coordinator_commands_tx.send(ChainsCoordinatorCommand::Terminate);
    if let Err(_e) = res {
        // Display log
    }
    let res = orchestrator_terminated_rx.recv();
    if let Err(_e) = res {
        // Display log
    }
    let _ = terminal.show_cursor();
    Ok(())
}
