use clarinet_deployments::onchain::TransactionStatus;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Cell, Row, Table};
use ratatui::Frame;

use super::App;

pub fn draw(f: &mut Frame, app: &mut App) {
    draw_contracts_status(f, app, f.area());
}

fn draw_contracts_status(f: &mut Frame, app: &mut App, area: Rect) {
    let rows = app.transactions.items.iter().map(|tx| {
        let (status, default_comment) = match &tx.status {
            TransactionStatus::Queued => ("🟪", "Transaction indexed".to_string()),
            TransactionStatus::Encoded(_, _) => {
                ("🟦", "Transaction encoded and queued".to_string())
            }
            TransactionStatus::Broadcasted(_, txid) => {
                ("🟨", format!("Transaction broadcasted (txid: {txid})"))
            }
            TransactionStatus::Confirmed => ("🟩", "Transaction confirmed".to_string()),
            TransactionStatus::Error(message) => ("🟥", message.to_string()),
        };

        Row::new(vec![
            Cell::from(status),
            Cell::from(tx.name.to_string()),
            Cell::from(default_comment),
        ])
        .height(1)
        .bottom_margin(0)
    });

    let t = Table::new(rows, vec![] as Vec<&Constraint>)
        .block(Block::default().title(format!("Broadcasting transactions to {}", app.node_url)))
        .style(Style::default().fg(Color::White))
        .widths([
            Constraint::Length(3),
            Constraint::Length(90),
            Constraint::Length(110),
        ]);
    f.render_widget(t, area);
}
