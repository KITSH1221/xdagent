//! Shared application and API data types.
use serde::{Deserialize, Serialize};

pub(crate) const API_BASE: &str = "http://127.0.0.1:8000";
pub(crate) const DEFAULT_CONVERSATION: &str = "default";

#[derive(Serialize)]
pub(crate) struct ChatRequest {
    pub(crate) message: String,
    pub(crate) conversation_id: String,
}

#[derive(Serialize)]
pub(crate) struct CreateConversationRequest {
    pub(crate) title: String,
    pub(crate) workspace_path: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct SwitchLeafRequest {
    pub(crate) message_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MessageNode {
    pub(crate) id: String,
    pub(crate) parent_id: Option<String>,
    pub(crate) role: String,
    pub(crate) content: String,
    #[allow(dead_code)]
    pub(crate) created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ConversationInfo {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) mode: String,
    pub(crate) workspace_path: Option<String>,
    pub(crate) active_leaf_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConversationListResponse {
    pub(crate) conversations: Vec<ConversationInfo>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConversationTreeResponse {
    pub(crate) conversation: ConversationInfo,
    pub(crate) messages: Vec<MessageNode>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConversationPathResponse {
    pub(crate) conversation_id: String,
    pub(crate) leaf_id: Option<String>,
    pub(crate) messages: Vec<MessageNode>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SwitchLeafResponse {
    #[allow(dead_code)]
    pub(crate) conversation_id: String,
    pub(crate) active_leaf_id: Option<String>,
    pub(crate) messages: Vec<MessageNode>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConfigStatusResponse {
    pub(crate) api_key_exist: bool,
    pub(crate) model: Option<String>,
    pub(crate) base_url: Option<String>,
}
pub(crate) struct Config {
    pub(crate) model_name: String,
    #[allow(unused)]
    pub(crate) base_url: String,
    pub(crate) api_key_exist: bool,
}

#[derive(Clone, Copy)]
pub(crate) enum Role {
    User,
    Assistant,
}

pub(crate) struct Message {
    pub(crate) role: Role,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum AppStatus {
    Ready,
    Loading,
    Thinking,
    Streaming,
    SwitchingBranch,
    Error,
}

impl AppStatus {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Loading => "loading",
            Self::Thinking => "thinking",
            Self::Streaming => "streaming",
            Self::SwitchingBranch => "switching",
            Self::Error => "error",
        }
    }

    pub(crate) fn can_interact(self) -> bool {
        matches!(self, Self::Ready | Self::Error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Focus {
    Conversations,
    Tree,
    Chat,
    Input,
}

impl Focus {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Conversations => Self::Tree,
            Self::Tree => Self::Chat,
            Self::Chat => Self::Input,
            Self::Input => Self::Conversations,
        }
    }
}

pub(crate) enum AppEvent {
    ConversationsLoaded(Vec<ConversationInfo>),
    HistoryLoaded {
        tree: ConversationTreeResponse,
        path: ConversationPathResponse,
    },
    BranchSwitched(SwitchLeafResponse),
    ConversationDeleted,
    HistoryCleared,
    AssistantChunk(String),
    AssistantDone,
    AssistantError(String),
    // 创建 workspace/general 会话成功后携带会话信息。
    ConversationCreated(ConversationInfo),

    ApprovalCompleted(String),
    ApprovalDenied(String),
}

#[derive(Debug, Deserialize)]
pub(crate) struct ApprovalResponse {
    pub(crate) approval_id: String,
    pub(crate) tool: String,
}
