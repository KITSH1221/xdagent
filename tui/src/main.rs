use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use futures_util::StreamExt;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    DefaultTerminal, Frame,
};
use serde::Serialize;

#[derive(Serialize)]
struct ChatRequest {
    message: String,
}

struct App {
    input: String,
    messages: Vec<Message>,
    chat_scroll: u16,
    status: String,
}

struct Message {
    role: Role,
    message: String,
}

enum Role {
    User,
    Assistant,
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

async fn send_chat_stream(
    app: &mut App,
    terminal: &mut DefaultTerminal,
    message: String,
) -> color_eyre::Result<()> {
    let client = reqwest::Client::new();

    let response = client
        .post("http://127.0.0.1:8000/chat/stream")
        .json(&ChatRequest { message })
        .send()
        .await?
        .error_for_status()?;

    app.messages.push(Message {
        role: Role::Assistant,
        message: String::new(),
    });

    app.status = "streaming".to_string();

    let mut stream = response.bytes_stream();
    let mut pending = Vec::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        pending.extend_from_slice(&chunk);

        if let Ok(text) = String::from_utf8(pending.clone()) {
            if let Some(last) = app.messages.last_mut() {
                last.message.push_str(&text);
            }

            pending.clear();
            app.chat_scroll = u16::MAX;
            terminal.draw(|frame| draw(frame, app))?;
        }
    }

    if !pending.is_empty() {
        let text = String::from_utf8_lossy(&pending);

        if let Some(last) = app.messages.last_mut() {
            last.message.push_str(&text);
        }
    }

    app.status = "ready".to_string();

    Ok(())
}

async fn run(mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
    let mut app = App {
        input: String::new(),
        messages: vec![],
        chat_scroll: 0,
        status: "ready".to_string(),
    };

    loop {
        terminal.draw(|frame| draw(frame, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Esc => break,

                KeyCode::Char(c) => {
                    app.input.push(c);
                }

                KeyCode::Backspace => {
                    app.input.pop();
                }

                KeyCode::Enter => {
                    let input = app.input.trim().to_string();

                    if input.is_empty() {
                        continue;
                    }

                    app.input.clear();

                    if input == "/clear" {
                        clear_history().await?;
                        app.messages.clear();
                        app.chat_scroll = 0;
                        app.status = "history cleared".to_string();
                        continue;
                    }

                    app.messages.push(Message {
                        role: Role::User,
                        message: input.clone(),
                    });

                    app.chat_scroll = 0;

                    if let Err(error) = send_chat_stream(&mut app, &mut terminal, input).await {
                        app.status = "error".to_string();

                        app.messages.push(Message {
                            role: Role::Assistant,
                            message: format!("Error: {error}"),
                        });
                    }
                }

                KeyCode::Up => {
                    app.chat_scroll = app.chat_scroll.saturating_sub(1);
                }

                KeyCode::Down => {
                    app.chat_scroll = app.chat_scroll.saturating_add(1);
                }

                KeyCode::PageUp => {
                    app.chat_scroll = app.chat_scroll.saturating_sub(5);
                }

                KeyCode::PageDown => {
                    app.chat_scroll = app.chat_scroll.saturating_add(5);
                }

                _ => {}
            }
        }
    }

    Ok(())
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

    let config_lines = vec![
        Line::from(Span::styled(" __   __  ____  ", Style::default().fg(Color::Cyan))),
        Line::from(Span::styled(" \\ \\ / / |  _ \\ ", Style::default().fg(Color::Cyan))),
        Line::from(Span::styled("  \\ V /  | | | |", Style::default().fg(Color::Cyan))),
        Line::from(Span::styled("  / . \\  | |_| |", Style::default().fg(Color::Cyan))),
        Line::from(Span::styled(" /_/ \\_\\ |____/ ", Style::default().fg(Color::Cyan))),
        Line::from(""),
        Line::from(vec![
            Span::styled("Provider: ", Style::default().fg(Color::Gray)),
            Span::styled("DeepSeek", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Model: ", Style::default().fg(Color::Gray)),
            Span::styled("deepseek-chat", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Gray)),
            Span::styled(&app.status, Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Messages: ", Style::default().fg(Color::Gray)),
            Span::styled(app.messages.len().to_string(), Style::default().fg(Color::White)),
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

    let chat_height = top[1].height.saturating_sub(2) as usize;
    let max_scroll = chat_lines.len().saturating_sub(chat_height) as u16;
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