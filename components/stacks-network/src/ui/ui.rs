use chainhook_sdk::chainhook_types::{
    StacksBlockData, StacksMicroblockData, StacksTransactionData,
};

use crate::{event::Status, log::LogLevel};

use super::{app::BlockData, App};
use tui::{
    backend::Backend,
    layout::{Constraint, Corner, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Tabs},
    Frame,
};

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let page_components = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(0),
                Constraint::Length(20),
                Constraint::Min(0),
                Constraint::Length(0),
            ]
            .as_ref(),
        )
        .split(f.size());

    let devnet_status_components = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(78)].as_ref())
        .split(page_components[1]);

    let service_len = match app.subnet_enabled {
        false => 7,
        true => 9,
    };

    let top_right_components = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(service_len), Constraint::Min(1)].as_ref())
        .split(devnet_status_components[1]);

    draw_devnet_status(f, app, devnet_status_components[0]);
    draw_services_status(f, app, top_right_components[0]);
    draw_mempool(f, app, top_right_components[1]);
    draw_blocks(f, app, page_components[2]);
    draw_help(f, app, page_components[3]);
}

fn draw_services_status<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let rows = app.services.items.iter().map(|service| {
        let status = match service.status {
            Status::Green => "🟩",
            Status::Yellow => "🟨",
            Status::Red => "🟥",
        };

        Row::new(vec![
            Cell::from(status),
            Cell::from(service.name.to_string()),
            Cell::from(service.comment.to_string()),
        ])
    });

    let t = Table::new(rows)
        .block(Block::default().borders(Borders::ALL).title("Services"))
        .style(Style::default().fg(Color::White))
        .widths(&[
            Constraint::Length(3),
            Constraint::Length(20),
            Constraint::Length(37),
        ]);
    f.render_widget(t, area);
}

fn draw_mempool<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let rows = app.mempool.items.iter().map(|item| {
        let cells = vec![Cell::from(item.tx_description.clone())];
        Row::new(cells).height(1).bottom_margin(0)
    });

    let t = Table::new(rows)
        .block(Block::default().borders(Borders::ALL).title("Mempool"))
        .style(Style::default().fg(Color::White))
        .widths(&[Constraint::Percentage(100)]);

    f.render_widget(t, area);
}

fn draw_devnet_status<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    // let page_components = Layout::default()
    //     .direction(Direction::Vertical)
    //     .constraints([Constraint::Length(20), Constraint::Min(1), Constraint::Length(5)].as_ref())
    //     .split(f.size());

    let logs: Vec<ListItem> = app
        .logs
        .items
        .iter()
        .rev()
        .map(|log| {
            // Log level
            let (style, label) = match log.level {
                LogLevel::Error => (Style::default().fg(Color::LightRed), "ERRO"),
                LogLevel::Warning => (Style::default().fg(Color::LightYellow), "WARN"),
                LogLevel::Info => (Style::default().fg(Color::LightBlue), "INFO"),
                LogLevel::Success => (Style::default().fg(Color::LightGreen), "INFO"),
                LogLevel::Debug => (Style::default().fg(Color::DarkGray), "DEBG"),
            };

            let log = Spans::from(vec![
                Span::styled(format!("{:<5}", label), style),
                Span::styled(&log.occurred_at, Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(log.message.clone(), Style::default().fg(Color::White)),
            ]);

            ListItem::new(vec![log])
        })
        .collect();
    let block = Block::default()
        .style(Style::default().fg(Color::White))
        .borders(Borders::ALL)
        .title("Stacks Devnet");
    let mut inner_area = block.inner(area);
    inner_area.height = inner_area.height.saturating_sub(1);
    f.render_widget(block, area);

    let logs_component = List::new(logs).start_corner(Corner::BottomLeft);
    f.render_widget(logs_component, inner_area);
}

fn draw_blocks<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let t = Table::new(vec![])
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .widths(&[]);
    f.render_widget(t, area);

    let blocks_components = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)].as_ref())
        .split(area);

    let titles = app.tabs.titles.iter().map(|s| s.clone()).collect();
    let blocks = Tabs::new(titles)
        .divider("")
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().bg(Color::White).fg(Color::Black))
        .block(Block::default().borders(Borders::NONE))
        .select(app.tabs.index);

    let block_details_components = Layout::default()
        .direction(Direction::Horizontal)
        .horizontal_margin(2)
        .vertical_margin(1)
        .constraints([Constraint::Length(75), Constraint::Min(1)].as_ref())
        .split(blocks_components[1]);

    f.render_widget(blocks, blocks_components[0]);

    if app.tabs.titles.is_empty() {
        return;
    }
    let transactions = match &app.blocks[(app.tabs.titles.len() - 1) - app.tabs.index] {
        BlockData::Block(selected_block) => {
            draw_block_details(f, block_details_components[0], &selected_block);
            &selected_block.transactions
        }
        BlockData::Microblock(selected_microblock) => {
            draw_microblock_details(f, block_details_components[0], &selected_microblock);
            &selected_microblock.transactions
        }
    };
    draw_transactions(f, block_details_components[1], &transactions);
}

fn draw_block_details<B>(f: &mut Frame<B>, area: Rect, block: &StacksBlockData)
where
    B: Backend,
{
    let paragraph = Paragraph::new(String::new()).block(
        Block::default()
            .borders(Borders::NONE)
            .style(Style::default().fg(Color::White))
            .title("Block Informations"),
    );
    f.render_widget(paragraph, area);

    let labels = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(2),
            ]
            .as_ref(),
        )
        .split(area);

    let label = "Block height:".to_string();
    let paragraph = Paragraph::new(label)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[1]);

    let value = format!("{}", block.block_identifier.index);
    let paragraph = Paragraph::new(value)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[2]);

    let label = "Block hash:".to_string();
    let paragraph = Paragraph::new(label)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[3]);

    let value = format!("{}", block.block_identifier.hash);
    let paragraph = Paragraph::new(value)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[4]);

    let label = "Bitcoin block height:".to_string();
    let paragraph = Paragraph::new(label)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[5]);

    let value = format!("{}", block.metadata.bitcoin_anchor_block_identifier.index);
    let paragraph = Paragraph::new(value)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[6]);

    let label = "Bitcoin block hash:".to_string();
    let paragraph = Paragraph::new(label)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[7]);

    let value = format!("{}", block.metadata.bitcoin_anchor_block_identifier.hash);
    let paragraph = Paragraph::new(value)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[8]);

    let label = "Pox Cycle:".to_string();
    let paragraph = Paragraph::new(label)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[9]);

    let value = format!("{}", block.metadata.pox_cycle_index);
    let paragraph = Paragraph::new(value)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[10]);

    // TODO(ludo): PoX informations
    // TODO(ludo): Mining informations (miner, VRF)
}

fn draw_microblock_details<B>(f: &mut Frame<B>, area: Rect, microblock: &StacksMicroblockData)
where
    B: Backend,
{
    let paragraph = Paragraph::new(String::new()).block(
        Block::default()
            .borders(Borders::NONE)
            .style(Style::default().fg(Color::White))
            .title("Microblock Informations"),
    );
    f.render_widget(paragraph, area);

    let labels = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(2),
            ]
            .as_ref(),
        )
        .split(area);

    let label = "Microblock height:".to_string();
    let paragraph = Paragraph::new(label)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[1]);

    let value = format!("{}", microblock.block_identifier.index);
    let paragraph = Paragraph::new(value)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[2]);

    let label = "Microblock hash:".to_string();
    let paragraph = Paragraph::new(label)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[3]);

    let value = format!("{}", microblock.block_identifier.hash);
    let paragraph = Paragraph::new(value)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[4]);

    let label = "Anchor block height:".to_string();
    let paragraph = Paragraph::new(label)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[5]);

    let value = format!("{}", microblock.metadata.anchor_block_identifier.index);
    let paragraph = Paragraph::new(value)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[6]);

    let label = "Anchor block hash:".to_string();
    let paragraph = Paragraph::new(label)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[7]);

    let value = format!("{}", microblock.metadata.anchor_block_identifier.hash);
    let paragraph = Paragraph::new(value)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, labels[8]);
}

fn draw_transactions<B>(f: &mut Frame<B>, area: Rect, transactions: &Vec<StacksTransactionData>)
where
    B: Backend,
{
    let transactions: Vec<ListItem> = transactions
        .iter()
        .map(|t| {
            let tx_info = Spans::from(vec![
                Span::styled(
                    match t.metadata.success {
                        true => "🟩",
                        false => "🟥",
                    },
                    Style::default(),
                ),
                Span::raw(" "),
                Span::styled(t.metadata.description.clone(), Style::default()),
                Span::raw(" "),
                Span::styled(t.metadata.result.clone(), Style::default()),
            ]);
            ListItem::new(vec![
                tx_info,
                // events,
            ])
        })
        .collect();

    let list = List::new(transactions)
        .block(
            Block::default()
                .borders(Borders::LEFT)
                .style(Style::default().fg(Color::White))
                .title("Transactions"),
        )
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("* ");
    let mut inner_area = area.clone();
    inner_area.height = inner_area.height.saturating_sub(1);
    f.render_widget(list, inner_area);
}

fn draw_help<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    // let help =
    //     " ⬅️  ➡️  Explore blocks          ⬆️  ⬇️  Explore transactions          0️⃣  Genesis Reset";
    let help = format!(" ⬅️  ➡️  Explore blocks          Path: {}", app.devnet_path);
    let paragraph = Paragraph::new(help.clone())
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(paragraph, area);
}
