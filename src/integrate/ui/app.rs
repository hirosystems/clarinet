use super::util::{StatefulList, TabsState};
use crate::integrate::{LogData, MempoolAdmissionData, ServiceStatusData};
use crate::types::{StacksBlockData, StacksTransactionData};
use tui::style::{Color, Style};
use tui::text::{Span, Spans};

pub struct App<'a> {
    pub title: &'a str,
    pub devnet_path: &'a str,
    pub should_quit: bool,
    pub blocks: Vec<StacksBlockData>,
    pub tabs: TabsState<'a>,
    pub transactions: StatefulList<StacksTransactionData>,
    pub mempool: StatefulList<MempoolAdmissionData>,
    pub logs: StatefulList<LogData>,
    pub services: StatefulList<ServiceStatusData>,
}

impl<'a> App<'a> {
    pub fn new(title: &'a str, devnet_path: &'a str) -> App<'a> {
        App {
            title,
            devnet_path,
            should_quit: false,
            tabs: TabsState::new(),
            blocks: vec![],
            transactions: StatefulList::with_items(vec![]),
            mempool: StatefulList::with_items(vec![]),
            logs: StatefulList::with_items(vec![]),
            services: StatefulList::with_items(vec![]),
        }
    }

    pub fn on_up(&mut self) {
        self.transactions.previous();
    }

    pub fn on_down(&mut self) {
        self.transactions.next();
    }

    pub fn on_right(&mut self) {
        self.tabs.next();
    }

    pub fn on_left(&mut self) {
        self.tabs.previous();
    }

    pub fn on_key(&mut self, c: char) {
        match c {
            'q' => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    pub fn on_tick(&mut self) {}

    pub fn reset(&mut self) {
        self.tabs = TabsState::new();
        self.blocks = vec![];
        self.transactions = StatefulList::with_items(vec![]);
        self.mempool = StatefulList::with_items(vec![]);
        self.logs = StatefulList::with_items(vec![]);
    }

    pub fn display_service_status_update(&mut self, service_update: ServiceStatusData) {
        let insertion_index = service_update.order;
        if insertion_index == self.services.items.len() {
            self.services.items.push(service_update)
        } else {
            self.services.items.remove(insertion_index);
            self.services.items.insert(insertion_index, service_update)
        }
    }

    pub fn display_log(&mut self, log: LogData) {
        use crate::integrate::LogLevel;
        use tracing::{debug, error, info, warn};
        match &log.level {
            LogLevel::Error => error!("{}", log.message),
            LogLevel::Warning => warn!("{}", log.message),
            LogLevel::Debug => debug!("{}", log.message),
            LogLevel::Info | &LogLevel::Success => info!("{}", log.message),
        }
        self.logs.items.push(log);
    }

    pub fn update_mempool(&mut self, tx: MempoolAdmissionData) {
        self.mempool.items.push(tx);
    }

    pub fn display_block(&mut self, block: StacksBlockData) {
        let cycle_len: u64 = block.metadata.pox_cycle_length.into();
        let abs_pos = block.metadata.bitcoin_anchor_block_identifier.index
            - block.metadata.bitcoin_genesis_block_identifier.index;
        let (start, end) = if abs_pos % cycle_len == (cycle_len - 1) {
            ("", "<")
        } else if abs_pos % cycle_len == 0 {
            (">", "")
        } else {
            ("", "")
        };
        let has_tx = if block.transactions.len() <= 1 {
            ""
        } else {
            "␂"
        };
        self.tabs.titles.push_front(Spans::from(Span::styled(
            format!(
                "{}[{}{}]{}",
                end, block.block_identifier.index, has_tx, start
            ),
            if block.metadata.pox_cycle_index % 2 == 1 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::LightYellow)
            },
        )));
        self.blocks.push(block);
        if self.tabs.index != 0 {
            self.tabs.index += 1;
        }
    }
}
