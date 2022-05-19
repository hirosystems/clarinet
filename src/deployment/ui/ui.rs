use crate::deployment::ContractStatus;

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
    let rows = app.contracts.items.iter().map(|contract| {
        let (status, default_comment) = match contract.status {
            ContractStatus::Queued => ("🟪", "Contract indexed".to_string()),
            ContractStatus::Encoded => ("🟦", "Contract encoded and queued".to_string()),
            ContractStatus::Broadcasted => ("🟨", "Contract broadcasted".to_string()),
            ContractStatus::Published => ("🟩", "Contract published".to_string()),
            ContractStatus::Error => ("🟥", "Error".to_string()),
        };

        Row::new(vec![
            Cell::from(status),
            Cell::from(contract.contract_id.to_string()),
            Cell::from(
                contract
                    .comment
                    .clone()
                    .unwrap_or(default_comment)
                    .to_string(),
            ),
        ])
        .height(1)
        .bottom_margin(0)
    });

    let t = Table::new(rows)
        .block(Block::default().title(format!("Publishing contracts using {}...", app.node_url)))
        .style(Style::default().fg(Color::White))
        .widths(&[
            Constraint::Length(3),
            Constraint::Length(80),
            Constraint::Length(120),
        ]);
    f.render_widget(t, area);
}
