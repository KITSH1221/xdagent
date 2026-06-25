//! Application state and keyboard/event handling.

use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

use crate::api::{
    spawn_chat_stream, spawn_clear_history, spawn_create_conversation, spawn_load_conversations,
    spawn_load_history, spawn_switch_branch,spawn_delete_conversation
};
use crate::types::{
    AppEvent, AppStatus, Config, ConversationInfo, DEFAULT_CONVERSATION, Focus, Message,
    MessageNode, Role,
};

pub(crate) struct App {
    pub(crate) input: String,
    pub(crate) messages: Vec<Message>,
    pub(crate) chat_scroll: u16,
    pub(crate) status: AppStatus,
    pub(crate) config: Config,
    pub(crate) conversation_id: String,
    pub(crate) conversation_title: String,
    pub(crate) conversations: Vec<ConversationInfo>,
    pub(crate) selected_conversation_index: usize,
    pub(crate) tree_nodes: Vec<MessageNode>,
    pub(crate) selected_tree_index: usize,
    pub(crate) active_leaf_id: Option<String>,
    pub(crate) focus: Focus,
}

impl App {
    pub(crate) fn new(config: Config) -> Self {
        Self {
            input: String::new(),
            messages: Vec::new(),
            chat_scroll: 0,
            status: AppStatus::Loading,
            config,
            conversation_id: DEFAULT_CONVERSATION.to_string(),
            conversation_title: "Default".to_string(),
            conversations: Vec::new(),
            selected_conversation_index: 0,
            tree_nodes: Vec::new(),
            selected_tree_index: 0,
            active_leaf_id: None,
            focus: Focus::Input,
        }
    }

    pub(crate) fn handle_key(
        &mut self,
        key: KeyEvent,
        tx: &mpsc::UnboundedSender<AppEvent>,
        max_scroll: u16,
    ) -> bool {
        match key.code {
            KeyCode::Esc => return false,
            KeyCode::Tab => self.focus = self.focus.next(),

            KeyCode::Up if self.focus == Focus::Conversations => {
                self.selected_conversation_index =
                    self.selected_conversation_index.saturating_sub(1);
            }
            KeyCode::Down if self.focus == Focus::Conversations => {
                if !self.conversations.is_empty() {
                    self.selected_conversation_index =
                        (self.selected_conversation_index + 1).min(self.conversations.len() - 1);
                }
            }
            KeyCode::Enter if self.focus == Focus::Conversations => {
                if !self.status.can_interact() {
                    return true;
                }
                if let Some(conversation) = self.conversations.get(self.selected_conversation_index)
                {
                    self.status = AppStatus::Loading;
                    spawn_load_history(conversation.id.clone(), tx.clone());
                }
            }

            KeyCode::Up if self.focus == Focus::Tree => {
                self.selected_tree_index = self.selected_tree_index.saturating_sub(1);
            }
            KeyCode::Down if self.focus == Focus::Tree => {
                if !self.tree_nodes.is_empty() {
                    self.selected_tree_index =
                        (self.selected_tree_index + 1).min(self.tree_nodes.len() - 1);
                }
            }
            KeyCode::Enter if self.focus == Focus::Tree => {
                if !self.status.can_interact() {
                    return true;
                }
                if let Some(node) = self.tree_nodes.get(self.selected_tree_index) {
                    self.status = AppStatus::SwitchingBranch;
                    spawn_switch_branch(self.conversation_id.clone(), node.id.clone(), tx.clone());
                }
            }

            KeyCode::Up if self.focus == Focus::Chat => {
                if self.chat_scroll == u16::MAX {
                    self.chat_scroll = max_scroll;
                }
                self.chat_scroll = self.chat_scroll.saturating_sub(1);
            }
            KeyCode::Down if self.focus == Focus::Chat => {
                if self.chat_scroll == u16::MAX {
                    self.chat_scroll = max_scroll;
                }
                self.chat_scroll = self.chat_scroll.saturating_add(1).min(max_scroll);
            }
            KeyCode::PageUp if self.focus == Focus::Chat => {
                self.chat_scroll = self.chat_scroll.saturating_sub(5);
            }
            KeyCode::PageDown if self.focus == Focus::Chat => {
                self.chat_scroll = self.chat_scroll.saturating_add(5).min(max_scroll);
            }

            KeyCode::Char(c) if self.focus == Focus::Input => self.input.push(c),
            KeyCode::Backspace if self.focus == Focus::Input => {
                self.input.pop();
            }
            KeyCode::Enter if self.focus == Focus::Input => {
                self.submit_input(tx);
            }
            _ => {}
        }

        true
    }

    fn submit_input(&mut self, tx: &mpsc::UnboundedSender<AppEvent>) {
        if !self.status.can_interact() {
            return;
        }

        let input = self.input.trim().to_string();
        if input.is_empty() {
            return;
        }
        self.input.clear();

        if let Some(path) = input.strip_prefix("/workspace ") {
            let path = path.trim().to_string();

            if path.is_empty() {
                return;
            }

            let title = std::path::Path::new(&path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Workspace")
                .to_string();

            self.status = AppStatus::Loading;

            spawn_create_conversation(title, Some(path), tx.clone());

            return;
        }

        if input == "/new" {
            self.status = AppStatus::Loading;

            spawn_create_conversation("New conversation".to_string(), None, tx.clone());

            return;
        }

        if let Some(title) = input.strip_prefix("/new ") {
            let title = title.trim();

            if title.is_empty() {
                return;
            }

            self.status = AppStatus::Loading;

            spawn_create_conversation(title.to_string(), None, tx.clone());

            return;
        }

        if let Some(conversation_id) = input.strip_prefix("/use ") {
            let conversation_id = conversation_id.trim();

            if conversation_id.is_empty() {
                return;
            }

            self.status = AppStatus::Loading;
            self.focus = Focus::Input;
            spawn_load_history(conversation_id.to_string(), tx.clone());

            return;
        }
        if input == "/delete" {
            let Some(conversation) = self.conversations.get(self.selected_conversation_index) else {
                return;
            };

            if conversation.id == DEFAULT_CONVERSATION {
                self.messages.push(Message {
                    role: Role::Assistant,
                    message: "Default conversation cannot be deleted.".to_string(),
                });
                return;
            }

            self.status = AppStatus::Loading;
            spawn_delete_conversation(conversation.id.clone(), tx.clone());

            return;
        }
        if input == "/clear" {
            self.status = AppStatus::Loading;
            spawn_clear_history(self.conversation_id.clone(), tx.clone());
            return;
        }

        self.messages.push(Message {
            role: Role::User,
            message: input.clone(),
        });
        self.messages.push(Message {
            role: Role::Assistant,
            message: String::new(),
        });
        self.chat_scroll = u16::MAX;
        self.status = AppStatus::Streaming;
        spawn_chat_stream(input, self.conversation_id.clone(), tx.clone());
    }

    pub(crate) fn handle_event(&mut self, event: AppEvent, tx: &mpsc::UnboundedSender<AppEvent>) {
        match event {
            AppEvent::ConversationsLoaded(conversations) => {
                self.conversations = conversations;
                self.selected_conversation_index = self
                    .conversations
                    .iter()
                    .position(|conversation| conversation.id == self.conversation_id)
                    .unwrap_or(0);
            }
            AppEvent::HistoryLoaded { tree, path } => {
                self.conversation_id = tree.conversation.id;
                self.conversation_title = tree.conversation.title;
                self.active_leaf_id = tree.conversation.active_leaf_id;
                self.tree_nodes = tree.messages;
                self.selected_tree_index = self
                    .active_leaf_id
                    .as_ref()
                    .and_then(|id| self.tree_nodes.iter().position(|node| &node.id == id))
                    .unwrap_or_else(|| self.tree_nodes.len().saturating_sub(1));
                self.conversation_id = path.conversation_id;
                self.active_leaf_id = path.leaf_id;
                self.messages = path.messages.into_iter().map(node_to_message).collect();
                self.chat_scroll = u16::MAX;
                self.status = AppStatus::Ready;
                spawn_load_conversations(tx.clone());
            }
            AppEvent::BranchSwitched(response) => {
                self.active_leaf_id = response.active_leaf_id;
                self.messages = response.messages.into_iter().map(node_to_message).collect();
                self.chat_scroll = u16::MAX;
                self.status = AppStatus::Ready;
            }
            AppEvent::ConversationCreated(conversation) => {
                self.conversation_id = conversation.id;
                self.conversation_title = conversation.title;

                self.messages.clear();
                self.tree_nodes.clear();
                self.selected_tree_index = 0;
                self.active_leaf_id = None;
                self.chat_scroll = 0;
                self.status = AppStatus::Loading;
                self.focus = Focus::Input;

                // 加载刚创建会话的空历史和树结构。
                spawn_load_history(self.conversation_id.clone(), tx.clone());
                spawn_load_conversations(tx.clone());
            }
            AppEvent::HistoryCleared => {
                self.messages.clear();
                self.tree_nodes.clear();
                self.selected_tree_index = 0;
                self.active_leaf_id = None;
                self.chat_scroll = 0;
                self.status = AppStatus::Ready;
                self.focus = Focus::Input;
            }
            AppEvent::AssistantChunk(text) => {
                if let Some(last) = self.messages.last_mut()
                    && matches!(last.role, Role::Assistant)
                {
                    last.message.push_str(&text);
                }
                self.chat_scroll = u16::MAX;
            }
            AppEvent::AssistantDone => {
                self.status = AppStatus::Loading;
                spawn_load_history(self.conversation_id.clone(), tx.clone());
            }
            AppEvent::AssistantError(error) => {
                self.status = AppStatus::Error;
                if let Some(last) = self.messages.last_mut()
                    && matches!(last.role, Role::Assistant)
                {
                    last.message.push_str(&format!("Error: {error}"));
                } else {
                    self.messages.push(Message {
                        role: Role::Assistant,
                        message: format!("Error: {error}"),
                    });
                }
            },
            AppEvent::ConversationDeleted => {
            self.conversation_id = DEFAULT_CONVERSATION.to_string();
            self.conversation_title = "Default".to_string();

            self.messages.clear();
            self.tree_nodes.clear();
            self.selected_tree_index = 0;
            self.active_leaf_id = None;
            self.chat_scroll = 0;
            self.status = AppStatus::Loading;
            self.focus = Focus::Conversations;

            spawn_load_conversations(tx.clone());
            spawn_load_history(self.conversation_id.clone(), tx.clone());
        }
        }
    }

    pub(crate) fn max_chat_scroll(&self, width: usize, height: usize) -> u16 {
        let line_count = if self.messages.is_empty() {
            1
        } else {
            self.messages
                .iter()
                .map(|message| {
                    let prefix_width = match message.role {
                        Role::User => 5,
                        Role::Assistant => 4,
                    };
                    let content_width = width.saturating_sub(prefix_width).max(1);
                    normalized_lines(&message.message)
                        .iter()
                        .map(|line| line.chars().count().max(1).div_ceil(content_width))
                        .sum::<usize>()
                })
                .sum()
        };

        line_count.saturating_sub(height) as u16
    }
}

fn node_to_message(node: MessageNode) -> Message {
    Message {
        role: if node.role == "user" {
            Role::User
        } else {
            Role::Assistant
        },
        message: node.content,
    }
}

pub(crate) fn normalized_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut previous_blank = false;

    for line in text.lines() {
        let is_blank = line.trim().is_empty();
        if is_blank {
            if !previous_blank {
                lines.push("");
            }
        } else {
            lines.push(line);
        }
        previous_blank = is_blank;
    }

    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    lines
}
