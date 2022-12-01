use clarinet_deployments::onchain::TransactionStatus;

use super::App;
use tui::{
    backend::Backend,
    layout::{Constraint, Rect},
    style::{Color, Style},
    widgets::{Block, Cell, Row, Table},
    Frame,
};

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    draw_contracts_status(f, app, f.size());
}

fn draw_contracts_status<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let rows = app.transactions.items.iter().map(|tx| {
        let (status, default_comment) = match &tx.status {
            TransactionStatus::Queued => ("🟪", "Transaction indexed".to_string()),
            TransactionStatus::Encoded(_, _) => {
                ("🟦", "Transaction encoded and queued".to_string())
            }
            TransactionStatus::Broadcasted(_) => ("🟨", "Transaction broadcasted".to_string()),
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

    let t = Table::new(rows)
        .block(Block::default().title(format!("Broadcasting transactions to {}", app.node_url)))
        .style(Style::default().fg(Color::White))
        .widths(&[
            Constraint::Length(3),
            Constraint::Length(80),
            Constraint::Length(120),
        ]);
    f.render_widget(t, area);
}
