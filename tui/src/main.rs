//! TUI entry point and event loop.

mod api;
mod app;
mod types;
mod ui;

use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;

use crate::api::{load_config, spawn_load_history};
use crate::app::App;
use crate::types::{AppEvent, Config};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = run(terminal).await;
    ratatui::restore();
    result
}

async fn run(mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();
    let config = load_config().await.unwrap_or_else(|_| Config {
        model_name: "unknown".to_string(),
        base_url: "offline".to_string(),
        api_key_exist: false,
    });
    let mut app = App::new(config);

    spawn_load_history(app.conversation_id.clone(), tx.clone());

    loop {
        let area = terminal.size()?;
        let chat_width = area.width.saturating_sub(32).saturating_sub(2) as usize;
        let chat_height = area.height.saturating_sub(5).saturating_sub(2) as usize;
        let max_scroll = app.max_chat_scroll(chat_width, chat_height);

        while let Ok(app_event) = rx.try_recv() {
            app.handle_event(app_event, &tx);
        }

        if event::poll(Duration::from_millis(16))? {
            loop {
                let Event::Key(key) = event::read()? else {
                    break;
                };

                if key.kind == KeyEventKind::Press && !app.handle_key(key, &tx, max_scroll) {
                    return Ok(());
                }

                if !event::poll(Duration::from_millis(0))? {
                    break;
                }
            }
        }

        terminal.draw(|frame| ui::draw(frame, &app))?;
    }
}
