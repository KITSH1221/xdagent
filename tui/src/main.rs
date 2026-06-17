use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    DefaultTerminal, Frame,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ChatRequest {
    message: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: String,
}

struct App {
    input: String,
    messages: Vec<Message>,
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

async fn send_chat(message: String) -> color_eyre::Result<String> {
    let client = reqwest::Client::new();

    let response = client
        .post("http://127.0.0.1:8000/chat")
        .json(&ChatRequest { message })
        .send()
        .await?
        .json::<ChatResponse>()
        .await?;

    Ok(response.message)
}

async fn run(mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
    let mut app = App {
        input: String::new(),
        messages: vec![],
    };

    loop {
        terminal.draw(|frame| draw(frame, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                        KeyCode::Char('q') => break,

                        KeyCode::Char(c) => {
                            app.input.push(c);
                        }

                        KeyCode::Backspace => {
                            app.input.pop();
                        }

                        KeyCode::Enter => {
                            if app.input.trim().is_empty() {
                                continue;
                            }

                            let question = app.input.clone();
                            app.input.clear();

                            app.messages.push(Message {
                                role: Role::User,
                                message: question.clone(),
                            });

                            let answer = send_chat(question).await?;

                            app.messages.push(Message {
                                role: Role::Assistant,
                                message: answer,
                            });
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

    let config = Paragraph::new(vec![
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
            Span::styled("ready", Style::default().fg(Color::Green)),
        ]),
    ])
    .block(
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

    let chat = Paragraph::new(chat_lines).block(
        Block::default()
            .title("Chat")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );

    let input = Paragraph::new(vec![Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Cyan)),
        Span::styled("Press Enter to send hello, q to quit", Style::default().fg(Color::DarkGray)),
    ])])
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