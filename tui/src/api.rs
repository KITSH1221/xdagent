//! Asynchronous HTTP tasks. Results are sent back through `AppEvent`.

use tokio::sync::mpsc;

use crate::types::{
    API_BASE, AppEvent, ApprovalResponse, ChatRequest, Config, ConfigStatusResponse,
    ConversationInfo, ConversationListResponse, ConversationPathResponse, ConversationTreeResponse,
    CreateConversationRequest, SwitchLeafRequest, SwitchLeafResponse,
};

pub(crate) async fn load_config() -> color_eyre::Result<Config> {
    let response = reqwest::Client::new()
        .get(format!("{API_BASE}/config/status"))
        .send()
        .await?
        .error_for_status()?
        .json::<ConfigStatusResponse>()
        .await?;

    Ok(Config {
        model_name: response.model.unwrap_or_else(|| "unknown".to_string()),
        base_url: response.base_url.unwrap_or_else(|| "unknown".to_string()),
        api_key_exist: response.api_key_exist,
    })
}

pub(crate) fn spawn_load_conversations(tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let result = reqwest::Client::new()
            .get(format!("{API_BASE}/conversations"))
            .send()
            .await
            .and_then(|response| response.error_for_status());

        let response = match result {
            Ok(response) => response,
            Err(error) => {
                let _ = tx.send(AppEvent::AssistantError(error.to_string()));
                return;
            }
        };

        match response.json::<ConversationListResponse>().await {
            Ok(response) => {
                let _ = tx.send(AppEvent::ConversationsLoaded(response.conversations));
            }
            Err(error) => {
                let _ = tx.send(AppEvent::AssistantError(error.to_string()));
            }
        }
    });
}

pub(crate) fn spawn_load_history(conversation_id: String, tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let result = async {
            let tree = client
                .get(format!("{API_BASE}/conversations/{conversation_id}/tree"))
                .send()
                .await?
                .error_for_status()?
                .json::<ConversationTreeResponse>()
                .await?;

            let path = client
                .get(format!("{API_BASE}/conversations/{conversation_id}/path"))
                .send()
                .await?
                .error_for_status()?
                .json::<ConversationPathResponse>()
                .await?;

            Ok::<_, reqwest::Error>((tree, path))
        }
        .await;

        match result {
            Ok((tree, path)) => {
                let _ = tx.send(AppEvent::HistoryLoaded { tree, path });
            }
            Err(error) => {
                let _ = tx.send(AppEvent::AssistantError(error.to_string()));
            }
        }
    });
}

pub(crate) fn spawn_switch_branch(
    conversation_id: String,
    message_id: String,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    tokio::spawn(async move {
        let response = reqwest::Client::new()
            .patch(format!(
                "{API_BASE}/conversations/{conversation_id}/active-leaf"
            ))
            .json(&SwitchLeafRequest {
                message_id: Some(message_id),
            })
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

        match response.json::<SwitchLeafResponse>().await {
            Ok(response) => {
                let _ = tx.send(AppEvent::BranchSwitched(response));
            }
            Err(error) => {
                let _ = tx.send(AppEvent::AssistantError(error.to_string()));
            }
        }
    });
}

pub(crate) fn spawn_clear_history(conversation_id: String, tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let response = reqwest::Client::new()
            .delete(format!("{API_BASE}/history"))
            .query(&[("conversation_id", conversation_id)])
            .send()
            .await;

        match response {
            Ok(response) => match response.error_for_status() {
                Ok(_) => {
                    let _ = tx.send(AppEvent::HistoryCleared);
                }
                Err(error) => {
                    let _ = tx.send(AppEvent::AssistantError(error.to_string()));
                }
            },
            Err(error) => {
                let _ = tx.send(AppEvent::AssistantError(error.to_string()));
            }
        }
    });
}

pub(crate) fn spawn_chat_stream(
    message: String,
    conversation_id: String,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    tokio::spawn(async move {
        let response = reqwest::Client::new()
            .post(format!("{API_BASE}/chat/stream"))
            .json(&ChatRequest {
                message,
                conversation_id,
            })
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

        let text = match response.text().await {
            Ok(text) => text,
            Err(error) => {
                let _ = tx.send(AppEvent::AssistantError(error.to_string()));
                return;
            }
        };

        for chunk in split_text_for_fake_stream(&text, 4) {
            let _ = tx.send(AppEvent::AssistantChunk(chunk));
            tokio::time::sleep(std::time::Duration::from_millis(12)).await;
        }

        let _ = tx.send(AppEvent::AssistantDone);
    });
}

pub(crate) fn spawn_create_conversation(
    title: String,
    workspace_path: Option<String>,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    tokio::spawn(async move {
        let result = async {
            let response = reqwest::Client::new()
                .post(format!("{API_BASE}/conversations"))
                .json(&CreateConversationRequest {
                    title,
                    workspace_path,
                })
                .send()
                .await?
                .error_for_status()?;

            response.json::<ConversationInfo>().await
        }
        .await;

        match result {
            Ok(conversation) => {
                let _ = tx.send(AppEvent::ConversationCreated(conversation));
            }
            Err(error) => {
                let _ = tx.send(AppEvent::AssistantError(error.to_string()));
            }
        }
    });
}

fn split_text_for_fake_stream(text: &str, chunk_size: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        current.push(ch);

        if current.chars().count() >= chunk_size {
            chunks.push(current);
            current = String::new();
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

pub(crate) fn spawn_delete_conversation(
    conversation_id: String,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    tokio::spawn(async move {
        let response = reqwest::Client::new()
            .delete(format!("{API_BASE}/conversations/{conversation_id}"))
            .send()
            .await;

        match response {
            Ok(response) => match response.error_for_status() {
                Ok(_) => {
                    let _ = tx.send(AppEvent::ConversationDeleted);
                }
                Err(error) => {
                    let _ = tx.send(AppEvent::AssistantError(error.to_string()));
                }
            },
            Err(error) => {
                let _ = tx.send(AppEvent::AssistantError(error.to_string()));
            }
        }
    });
}

pub(crate) fn spawn_approve(approval_id: String, tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let response = reqwest::Client::new()
            .post(format!("{API_BASE}/approvals/{approval_id}/approve"))
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

        match response.json::<ApprovalResponse>().await {
            Ok(response) => {
                let _ = tx.send(AppEvent::ApprovalCompleted(format!(
                    "Approved and executed `{}`. approval_id: {}",
                    response.tool, response.approval_id
                )));
            }
            Err(error) => {
                let _ = tx.send(AppEvent::AssistantError(error.to_string()));
            }
        }
    });
}

pub(crate) fn spawn_deny(approval_id: String, tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let response = reqwest::Client::new()
            .post(format!("{API_BASE}/approvals/{approval_id}/deny"))
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

        match response.json::<ApprovalResponse>().await {
            Ok(response) => {
                let _ = tx.send(AppEvent::ApprovalDenied(format!(
                    "Denied `{}`. approval_id: {}",
                    response.tool, response.approval_id
                )));
            }
            Err(error) => {
                let _ = tx.send(AppEvent::AssistantError(error.to_string()));
            }
        }
    });
}
