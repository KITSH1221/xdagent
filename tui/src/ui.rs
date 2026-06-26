//! Ratatui layout and rendering helpers.

use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

use crate::app::{App, normalized_lines};
use crate::types::{AppStatus, ConversationInfo, Focus, MessageNode, Role};

pub(crate) fn draw(frame: &mut Frame, app: &App) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(frame.area());
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(32), Constraint::Min(0)])
        .split(vertical[0]);
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9),
            Constraint::Length(2),
            Constraint::Length(9),
            Constraint::Length(2),
            Constraint::Min(5),
        ])
        .split(top[0]);

    let config = render_config(app);
    let conversations = render_conversations(app, left[2].height.saturating_sub(1) as usize);
    let tree = render_tree(app, left[4].height.saturating_sub(1) as usize);
    let chat = render_chat(app, top[1].width, top[1].height);
    let input = render_input(app);

    frame.render_widget(config, left[0]);
    frame.render_widget(conversations, left[2]);
    frame.render_widget(tree, left[4]);
    frame.render_widget(chat, top[1]);
    frame.render_widget(input, vertical[1]);
}

fn render_config(app: &App) -> Paragraph<'static> {
    let status_color = match app.status {
        AppStatus::Ready => Color::Green,
        AppStatus::Thinking | AppStatus::Streaming => Color::Yellow,
        AppStatus::Loading | AppStatus::SwitchingBranch => Color::Cyan,
        AppStatus::Error => Color::Red,
    };

    let status_label = match app.status {
        AppStatus::Thinking => format!("{} thinking", spinner_frame()),
        AppStatus::Loading => format!("{} loading", spinner_frame()),
        AppStatus::SwitchingBranch => format!("{} switching", spinner_frame()),
        _ => app.status.label().to_string(),
    };

    let lines = vec![
        section_title("CONFIG", 28, false),
        Line::from(Span::styled("XD coder", Style::default().fg(Color::Cyan))),
        Line::from(format!("model : {}", app.config.model_name)),
        Line::from(vec![
            Span::raw("status: "),
            Span::styled(status_label, Style::default().fg(status_color)),
        ]),
        Line::from(format!(
            "api key: {}",
            if app.config.api_key_exist {
                "set"
            } else {
                "missing"
            }
        )),
        Line::from(format!(
            "approval: {}",
            if app.pending_approval_id.is_some() {
                "pending"
            } else {
                "none"
            }
        )),
        Line::from("/new /workspace"),
        Line::from("/clear /delete /use"),
    ];

    Paragraph::new(lines).wrap(Wrap { trim: false })
}

fn render_conversations(app: &App, height: usize) -> Paragraph<'static> {
    let scroll = if app.selected_conversation_index >= height && height > 0 {
        (app.selected_conversation_index - height + 1) as u16
    } else {
        0
    };

    let mut lines = vec![section_title(
        "CONVERSATIONS",
        28,
        app.focus == Focus::Conversations,
    )];
    lines.extend(conversation_lines(app));

    Paragraph::new(lines)
        .scroll((scroll, 0))
        .wrap(Wrap { trim: false })
}

fn render_tree(app: &App, height: usize) -> Paragraph<'static> {
    let scroll = if app.selected_tree_index >= height && height > 0 {
        (app.selected_tree_index - height + 1) as u16
    } else {
        0
    };

    let mut lines = vec![section_title(
        &format!("TREE: {}", app.conversation_title),
        28,
        app.focus == Focus::Tree,
    )];
    lines.extend(tree_lines(app));

    Paragraph::new(lines)
        .scroll((scroll, 0))
        .wrap(Wrap { trim: false })
}

fn render_chat(app: &App, width: u16, height: u16) -> Paragraph<'static> {
    let mut lines = vec![section_title(
        "CHAT",
        width.saturating_sub(2) as usize,
        app.focus == Focus::Chat,
    )];

    for message in &app.messages {
        let (label, label_color, text_color) = match message.role {
            Role::User => ("You: ", Color::Cyan, Color::White),
            Role::Assistant => ("AI: ", Color::Green, Color::Gray),
        };
        let mut message_lines = normalized_lines(&message.message).into_iter();
        if let Some(first_line) = message_lines.next() {
            lines.push(Line::from(vec![
                Span::styled(label, Style::default().fg(label_color)),
                Span::styled(first_line.to_string(), Style::default().fg(text_color)),
            ]));
            for line in message_lines {
                lines.push(Line::from(vec![
                    Span::raw(" ".repeat(label.len())),
                    Span::styled(line.to_string(), Style::default().fg(text_color)),
                ]));
            }
        }
    }
    if app.messages.is_empty() {
        lines.push(Line::from(Span::styled(
            "(no messages)",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let content_width = width as usize;
    let content_height = height.saturating_sub(1) as usize;
    let scroll = app
        .chat_scroll
        .min(app.max_chat_scroll(content_width, content_height));

    Paragraph::new(lines)
        .scroll((scroll, 0))
        .wrap(Wrap { trim: false })
}

fn render_input(app: &App) -> Paragraph<'_> {
    Paragraph::new(Line::from(vec![
        Span::styled(
            "> ",
            Style::default()
                .fg(if app.focus == Focus::Input {
                    Color::Yellow
                } else {
                    Color::Rgb(240, 240, 240)
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(&app.input, Style::default().fg(Color::White)),
    ]))
    .wrap(Wrap { trim: false })
}

fn section_title(title: &str, width: usize, focused: bool) -> Line<'static> {
    let title_color = if focused {
        Color::Yellow
    } else {
        Color::Rgb(240, 240, 240)
    };
    let line_len = width
        .saturating_sub(title.chars().count())
        .saturating_sub(2)
        .max(3);

    Line::from(vec![
        Span::styled(title.to_string(), Style::default().fg(title_color)),
        Span::raw(" "),
        Span::styled(
            "━".repeat(line_len),
            Style::default().fg(Color::Rgb(240, 240, 240)),
        ),
    ])
}

fn conversation_lines(app: &App) -> Vec<Line<'static>> {
    if app.conversations.is_empty() {
        return vec![Line::from(Span::styled(
            "(no conversations)",
            Style::default().fg(Color::DarkGray),
        ))];
    }

    app.conversations
        .iter()
        .enumerate()
        .map(|(index, conversation)| conversation_line(app, index, conversation))
        .collect()
}

fn conversation_line(app: &App, index: usize, conversation: &ConversationInfo) -> Line<'static> {
    let cursor = if index == app.selected_conversation_index {
        Span::styled(
            ">",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw(" ")
    };
    let active = if conversation.id == app.conversation_id {
        Span::styled("*", Style::default().fg(Color::Green))
    } else {
        Span::raw(" ")
    };
    let mode_color = if conversation.workspace_path.is_some() {
        Color::Magenta
    } else {
        Color::Blue
    };
    let mode = if conversation.workspace_path.is_some() {
        "W"
    } else {
        conversation.mode.chars().next().map_or("G", |_| "G")
    };
    let title = conversation.title.chars().take(14).collect::<String>();

    Line::from(vec![
        cursor,
        Span::raw(" "),
        active,
        Span::raw(" "),
        Span::styled(format!("[{mode}] "), Style::default().fg(mode_color)),
        Span::styled(title, Style::default().fg(Color::Gray)),
    ])
}

fn tree_lines(app: &App) -> Vec<Line<'static>> {
    if app.tree_nodes.is_empty() {
        return vec![Line::from(Span::styled(
            "(no messages)",
            Style::default().fg(Color::DarkGray),
        ))];
    }

    app.tree_nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let cursor = if index == app.selected_tree_index {
                Span::styled(
                    ">",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw(" ")
            };
            let active = if app.active_leaf_id.as_deref() == Some(node.id.as_str()) {
                Span::styled("*", Style::default().fg(Color::Green))
            } else {
                Span::raw(" ")
            };
            let (role, role_color) = if node.role == "user" {
                ("You", Color::Cyan)
            } else {
                ("AI", Color::Green)
            };
            let depth = branch_depth(&app.tree_nodes, node);
            let branch = branch_marker(&app.tree_nodes, index, node);
            let preview = node
                .content
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(10)
                .collect::<String>();
            Line::from(vec![
                cursor,
                Span::styled(
                    "  ".repeat(depth.saturating_sub(1)),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(branch, Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                active,
                Span::raw(" "),
                Span::styled(role, Style::default().fg(role_color)),
                Span::raw(": "),
                Span::styled(preview, Style::default().fg(Color::Gray)),
            ])
        })
        .collect()
}

// Linear messages stay flat; indentation grows only at a real fork.
fn branch_depth(nodes: &[MessageNode], node: &MessageNode) -> usize {
    let mut depth = 0;
    let mut traversed = 0;
    let mut parent_id = node.parent_id.as_ref();
    while let Some(id) = parent_id {
        let Some(parent) = nodes.iter().find(|candidate| &candidate.id == id) else {
            break;
        };
        if child_count(nodes, &parent.id) > 1 {
            depth += 1;
        }
        traversed += 1;
        if traversed >= nodes.len() {
            break;
        }
        parent_id = parent.parent_id.as_ref();
    }
    depth
}

fn child_count(nodes: &[MessageNode], parent_id: &str) -> usize {
    nodes
        .iter()
        .filter(|node| node.parent_id.as_deref() == Some(parent_id))
        .count()
}

fn branch_marker(nodes: &[MessageNode], index: usize, node: &MessageNode) -> &'static str {
    let Some(parent_id) = node.parent_id.as_deref() else {
        return "  ";
    };
    if child_count(nodes, parent_id) <= 1 {
        return "  ";
    }

    let has_later_sibling = nodes
        .iter()
        .skip(index + 1)
        .any(|candidate| candidate.parent_id.as_deref() == Some(parent_id));
    if has_later_sibling { "|-" } else { "`-" }
}

fn spinner_frame() -> &'static str {
    const FRAMES: [&str; 4] = ["-", "\\", "|", "/"];

    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);

    FRAMES[((millis / 120) as usize) % FRAMES.len()]
}
