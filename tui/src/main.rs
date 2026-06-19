use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use futures_util::StreamExt;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Serialize)]
struct ChatRequest {
    message: String,
}

struct App {
    input: String,
    messages: Vec<Message>,
    chat_scroll: u16,
    status: String,
    config: Config,
}

struct Message {
    role: Role,
    message: String,
}

enum Role {
    User,
    Assistant,
}

enum AppEvent {
    AssistantChunk(String),
    AssistantDone,
    AssistantError(String),
}
#[derive(Debug, Deserialize)]
struct ConfigStatusResponse {
    api_key_exist: bool,
    model: Option<String>,
    base_url: Option<String>,
}
struct Config {
    model_name: String,
    base_url: String,
    api_key_exist: bool,
}

impl Config {
    async fn new() -> color_eyre::Result<Self> {
        let client = reqwest::Client::new();

        let response = client
            .get("http://127.0.0.1:8000/config/status")
            .send()
            .await?
            .error_for_status()?
            .json::<ConfigStatusResponse>()
            .await?;

        Ok(Self {
            model_name: response.model.unwrap_or_else(|| "unknown".to_string()),
            base_url: response.base_url.unwrap_or_else(|| "unknown".to_string()),
            api_key_exist: response.api_key_exist,
        })
    }
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let terminal = ratatui::init();
    let result = run(terminal).await;
    ratatui::restore();

    result
}

async fn clear_history() -> color_eyre::Result<()> {
    let client = reqwest::Client::new();

    client
        .delete("http://127.0.0.1:8000/history")
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

fn spawn_chat_stream(message: String, tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();

        let response = client
            .post("http://127.0.0.1:8000/chat/stream")
            .json(&ChatRequest { message })
            .send()
            .await;

        let response = match response {
            Ok(response) => match response.error_for_status() {
                Ok(response) => response,
                Err(error) => {
                    let _ = tx.send(AppEvent::AssistantError(error.to_string()));
                    return;
                }
            },
            Err(error) => {
                let _ = tx.send(AppEvent::AssistantError(error.to_string()));
                return;
            }
        };

        let mut stream = response.bytes_stream();
        let mut pending = Vec::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(chunk) => {
                    pending.extend_from_slice(&chunk);

                    if let Ok(text) = String::from_utf8(pending.clone()) {
                        let _ = tx.send(AppEvent::AssistantChunk(text));
                        pending.clear();
                    }
                }
                Err(error) => {
                    let _ = tx.send(AppEvent::AssistantError(error.to_string()));
                    return;
                }
            }
        }

        if !pending.is_empty() {
            let text = String::from_utf8_lossy(&pending).to_string();
            let _ = tx.send(AppEvent::AssistantChunk(text));
        }

        let _ = tx.send(AppEvent::AssistantDone);
    });
}

async fn run(mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    let config = Config::new().await.unwrap_or_else(|_| Config {
        model_name: "unknown".to_string(),
        base_url: "offline".to_string(),
        api_key_exist: false,
    });

    let mut app = App {
        input: String::new(),
        messages: vec![],
        chat_scroll: 0,
        status: "ready".to_string(),
        config,
    };

    loop {
        let area = terminal.size()?;
        let chat_width = area.width.saturating_sub(28).saturating_sub(2) as usize;
        let chat_height = area.height.saturating_sub(5).saturating_sub(2) as usize;
        let max_scroll = max_chat_scroll(&app, chat_width, chat_height);

        while let Ok(app_event) = rx.try_recv() {
            handle_app_event(&mut app, app_event);
        }

        if event::poll(Duration::from_millis(16))? {
            loop {
                let Event::Key(key) = event::read()? else {
                    break;
                };

                if key.kind == KeyEventKind::Press {
                    if !handle_key(&mut app, key, &tx, max_scroll) {
                        return Ok(());
                    }
                }

                if !event::poll(Duration::from_millis(0))? {
                    break;
                }
            }
        }

        terminal.draw(|frame| draw(frame, &app))?;
    }
}

fn handle_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    tx: &mpsc::UnboundedSender<AppEvent>,
    max_scroll: u16,
) -> bool {
    match key.code {
        KeyCode::Esc => return false,

        KeyCode::Char(c) => {
            app.input.push(c);
        }

        KeyCode::Backspace => {
            app.input.pop();
        }

        KeyCode::Enter => {
            let input = app.input.trim().to_string();

            if input.is_empty() {
                return true;
            }

            app.input.clear();

            if input == "/clear" {
                app.messages.clear();
                app.chat_scroll = 0;
                app.status = "history cleared".to_string();

                tokio::spawn(async {
                    let _ = clear_history().await;
                });
                return true;
            }

            if app.status == "streaming" {
                app.status = "busy".to_string();
                return true;
            }

            app.messages.push(Message {
                role: Role::User,
                message: input.clone(),
            });

            app.messages.push(Message {
                role: Role::Assistant,
                message: String::new(),
            });

            app.chat_scroll = u16::MAX;
            app.status = "streaming".to_string();

            spawn_chat_stream(input, tx.clone());
        }

        KeyCode::Up => {
            if app.chat_scroll == u16::MAX {
                app.chat_scroll = max_scroll;
            }

            app.chat_scroll = app.chat_scroll.saturating_sub(1);
        }

        KeyCode::Down => {
            if app.chat_scroll == u16::MAX {
                app.chat_scroll = max_scroll;
            }

            app.chat_scroll = app.chat_scroll.saturating_add(1).min(max_scroll);
        }

        KeyCode::PageUp => {
            if app.chat_scroll == u16::MAX {
                app.chat_scroll = max_scroll;
            }

            app.chat_scroll = app.chat_scroll.saturating_sub(5);
        }

        KeyCode::PageDown => {
            if app.chat_scroll == u16::MAX {
                app.chat_scroll = max_scroll;
            }

            app.chat_scroll = app.chat_scroll.saturating_add(5).min(max_scroll);
        }
        _ => {}
    }

    true
}
fn handle_app_event(app: &mut App, app_event: AppEvent) {
    match app_event {
        AppEvent::AssistantChunk(text) => {
            if let Some(last) = app.messages.last_mut() {
                if matches!(last.role, Role::Assistant) {
                    last.message.push_str(&text);
                }
            }

            app.chat_scroll = u16::MAX;
        }

        AppEvent::AssistantDone => {
            app.status = "ready".to_string();
        }

        AppEvent::AssistantError(error) => {
            app.status = "error".to_string();

            if let Some(last) = app.messages.last_mut() {
                if matches!(last.role, Role::Assistant) {
                    last.message.push_str(&format!("Error: {error}"));
                }
            }
        }
    }
}

fn max_chat_scroll(app: &App, chat_width: usize, chat_height: usize) -> u16 {
    let line_count = if app.messages.is_empty() {
        1
    } else {
        app.messages
            .iter()
            .map(|message| {
                let prefix_width = match message.role {
                    Role::User => 5,
                    Role::Assistant => 4,
                };
                let content_width = chat_width.saturating_sub(prefix_width).max(1);
                let message_width = message.message.chars().count().max(1);

                message_width.div_ceil(content_width)
            })
            .sum()
    };

    line_count.saturating_sub(chat_height) as u16
}

fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(0)])
        .split(vertical[0]);

    let status_color = match app.status.as_str() {
        "ready" => Color::Green,
        "streaming" => Color::Yellow,
        "error" => Color::Red,
        _ => Color::Gray,
    };

    let config_lines = vec![
        Line::from(Span::styled(
            " __   __  ____  ",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            " \\ \\ / / |  _ \\ ",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            "  \\ V /  | | | |",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            "  / . \\  | |_| |",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            " /_/ \\_\\ |____/ ",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Base_url:", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", &app.config.base_url.to_string()),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("Model: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", &app.config.model_name.to_string()),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Gray)),
            Span::styled(&app.status, Style::default().fg(status_color)),
        ]),
        Line::from(vec![
            Span::styled("Messages: ", Style::default().fg(Color::Gray)),
            Span::styled(
                app.messages.len().to_string(),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("Keys", Style::default().fg(Color::DarkGray))),
        Line::from("Enter send"),
        Line::from("Up/Down scroll"),
        Line::from("/clear clear"),
        Line::from("Esc quit"),
    ];

    let config = Paragraph::new(config_lines).block(
        Block::default()
            .title("Config")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    let mut chat_lines = Vec::new();

    for message in &app.messages {
        match message.role {
            Role::User => {
                chat_lines.push(Line::from(vec![
                    Span::styled("You: ", Style::default().fg(Color::Cyan)),
                    Span::styled(&message.message, Style::default().fg(Color::White)),
                ]));
            }
            Role::Assistant => {
                chat_lines.push(Line::from(vec![
                    Span::styled("AI: ", Style::default().fg(Color::Green)),
                    Span::styled(&message.message, Style::default().fg(Color::Gray)),
                ]));
            }
        }
    }

    if chat_lines.is_empty() {
        chat_lines.push(Line::from(vec![
            Span::styled("AI: ", Style::default().fg(Color::Green)),
            Span::styled("Hi, how can I help?", Style::default().fg(Color::Gray)),
        ]));
    }

    let chat_width = top[1].width.saturating_sub(2) as usize;
    let chat_height = top[1].height.saturating_sub(2) as usize;
    let max_scroll = max_chat_scroll(app, chat_width, chat_height);
    let chat_scroll = app.chat_scroll.min(max_scroll);

    let chat = Paragraph::new(chat_lines)
        .scroll((chat_scroll, 0))
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title("Chat")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );

    let input = Paragraph::new(vec![Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Cyan)),
        Span::styled(&app.input, Style::default().fg(Color::White)),
    ])])
    .wrap(Wrap { trim: false })
    .block(
        Block::default()
            .title("Input")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(config, top[0]);
    frame.render_widget(chat, top[1]);
    frame.render_widget(input, vertical[1]);
}
