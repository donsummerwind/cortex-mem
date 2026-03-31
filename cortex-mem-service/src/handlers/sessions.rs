use axum::{
    Json,
    extract::{Path, State},
};
use std::{path::PathBuf, sync::Arc, time::Duration};

use crate::{
    error::{AppError, Result},
    models::{
        AddMessageRequest, ApiResponse, CloseAndWaitRequest, CloseAndWaitResponse,
        SessionResponse,
    },
    state::AppState,
};
use cortex_mem_core::{
    ChangeType,
    MemoryIndex,
    SessionExtractionSummary,
    VectorSyncManager,
    session::SessionMetadata,
    types::ContextLayer,
    vector_store::uri_to_vector_id,
};

/// Create a new session
pub async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<SessionResponse>>> {
    let thread_id = payload.get("thread_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let title = payload.get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let user_id = payload.get("user_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let agent_id = payload.get("agent_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let session_mgr = state.current_session_manager().await;
    let session_mgr = session_mgr.write().await;
    let mut metadata = session_mgr.create_session_with_ids(&thread_id, user_id, agent_id).await?;

    // Set title if provided
    if let Some(t) = title {
        metadata.set_title(t);
        session_mgr.update_session(&metadata).await?;
    }

    let response = SessionResponse {
        thread_id: metadata.thread_id,
        status: format!("{:?}", metadata.status),
        message_count: metadata.message_count,
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// List all sessions
pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<SessionResponse>>>> {
    // Get tenant root if set
    let tenant_root = state.current_tenant_root.read().await.clone();

    // Build the path
    let session_path = if let Some(root) = tenant_root {
        root.join("session")
    } else {
        // 直接使用 data_dir 作为根目录（不再添加 cortex 子目录）
        state.data_dir.join("session")
    };

    tracing::debug!("Listing sessions from: {:?}", session_path);

    if !session_path.exists() {
        return Ok(Json(ApiResponse::success(vec![])));
    }

    let mut sessions = Vec::new();
    if let Ok(dir) = std::fs::read_dir(&session_path) {
        for entry in dir.flatten() {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let thread_id = entry.file_name().to_string_lossy().to_string();

                // Skip hidden directories
                if thread_id.starts_with('.') {
                    continue;
                }

                // Try to load session metadata directly from file
                let metadata_path = entry.path().join(".session.json");
                if metadata_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&metadata_path) {
                        if let Ok(metadata) = serde_json::from_str::<SessionMetadata>(&content) {
                            sessions.push(SessionResponse {
                                thread_id: metadata.thread_id,
                                status: format!("{:?}", metadata.status),
                                message_count: metadata.message_count,
                                created_at: metadata.created_at,
                                updated_at: metadata.updated_at,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(Json(ApiResponse::success(sessions)))
}

/// Add message to session
pub async fn add_message(
    State(state): State<Arc<AppState>>,
    Path(thread_id): Path<String>,
    Json(payload): Json<AddMessageRequest>,
) -> Result<Json<ApiResponse<String>>> {
    use cortex_mem_core::MessageRole;

    let role = match payload.role.to_lowercase().as_str() {
        "user" => MessageRole::User,
        "assistant" => MessageRole::Assistant,
        "system" => MessageRole::System,
        _ => return Err(crate::error::AppError::BadRequest(
            format!("Invalid role: {}", payload.role)
        )),
    };

    // Ensure the session exists before adding a message (auto-create if missing)
    {
        let session_mgr = state.current_session_manager().await;
        let session_mgr = session_mgr.read().await;
        if session_mgr.load_session(&thread_id).await.is_err() {
            drop(session_mgr);
            let session_mgr = state.current_session_manager().await;
            let session_mgr = session_mgr.write().await;
            session_mgr.create_session_with_ids(&thread_id, None, None).await?;
            tracing::info!("Auto-created session '{}' on first message", thread_id);
        }
    }

    // Use SessionManager::add_message to trigger MemoryEventCoordinator events
    // This ensures proper event chain for automatic indexing and layer generation
    let session_mgr = state.current_session_manager().await;
    let session_mgr = session_mgr.read().await;
    let message = session_mgr.add_message(&thread_id, role, payload.content).await?;
    drop(session_mgr);

    // Build message URI (matches what MessageStorage actually writes)
    let message_uri = format!(
        "cortex://session/{}/timeline/{}/{}/{}_{}.md",
        thread_id,
        message.timestamp.format("%Y-%m"),
        message.timestamp.format("%d"),
        message.timestamp.format("%H_%M_%S"),
        &message.id[..8]
    );

    // Emit LayerUpdateNeeded so the tenant-aware MemoryEventCoordinator
    // (re)generates L0/L1 layer summaries for the session's timeline directory.
    // VectorSyncNeeded is handled automatically by AutomationManager (via SessionManager's
    // MessageAdded event → CortexEvent → AutomationManager::index_session_l2).
    {
        use cortex_mem_core::memory_events::{ChangeType, MemoryEvent};
        use cortex_mem_core::memory_index::MemoryScope;

        let tx_guard = state.memory_event_tx.read().await;
        if let Some(ref tx) = *tx_guard {
            let day_dir_uri = format!(
                "cortex://session/{}/timeline/{}/{}",
                thread_id,
                message.timestamp.format("%Y-%m"),
                message.timestamp.format("%d"),
            );
            match tx.send(MemoryEvent::LayerUpdateNeeded {
                scope: MemoryScope::Session,
                owner_id: thread_id.clone(),
                directory_uri: day_dir_uri,
                change_type: ChangeType::Add,
                changed_file: message_uri.clone(),
            }) {
                Ok(_) => tracing::info!("📤 Dispatched LayerUpdateNeeded for session {}", thread_id),
                Err(e) => tracing::error!("❌ Failed to dispatch LayerUpdateNeeded: {}", e),
            }
        } else {
            tracing::warn!("⚠️ No memory_event_tx available, skipping event dispatch");
        }
    }

    Ok(Json(ApiResponse::success(format!("Message saved to {}", message_uri))))
}

/// Close session
pub async fn close_session(
    State(state): State<Arc<AppState>>,
    Path(thread_id): Path<String>,
) -> Result<Json<ApiResponse<SessionResponse>>> {
    let session_mgr = state.current_session_manager().await;
    let mut session_mgr = session_mgr.write().await;
    let metadata = session_mgr.close_session(&thread_id).await?;
    drop(session_mgr);

    let response = SessionResponse {
        thread_id: metadata.thread_id,
        status: format!("{:?}", metadata.status),
        message_count: metadata.message_count,
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// Close session and wait until extracted memories are ready for retrieval.
pub async fn close_session_and_wait(
    State(state): State<Arc<AppState>>,
    Path(thread_id): Path<String>,
    payload: Option<Json<CloseAndWaitRequest>>,
) -> Result<Json<ApiResponse<CloseAndWaitResponse>>> {
    let request = payload
        .map(|Json(v)| v)
        .unwrap_or(CloseAndWaitRequest {
            timeout_secs: 120,
            poll_interval_ms: 500,
        });

    if request.timeout_secs == 0 {
        return Err(AppError::BadRequest("timeout_secs must be greater than 0".to_string()));
    }
    if request.poll_interval_ms == 0 {
        return Err(AppError::BadRequest("poll_interval_ms must be greater than 0".to_string()));
    }

    let start = tokio::time::Instant::now();
    let timeout = Duration::from_secs(request.timeout_secs);
    let poll = Duration::from_millis(request.poll_interval_ms);

    let session_mgr = state.current_session_manager().await;
    let mut session_mgr = session_mgr.write().await;
    let metadata = session_mgr.close_session(&thread_id).await?;
    drop(session_mgr);

    let user_id = metadata.user_id.clone().unwrap_or_else(|| "default".to_string());
    let agent_id = metadata.agent_id.clone().unwrap_or_else(|| "default".to_string());

    loop {
        let status = collect_close_wait_status(&state, &thread_id, &user_id, &agent_id, start).await?;
        if is_close_wait_ready(&status) {
            return Ok(Json(ApiResponse::success(status)));
        }

        if start.elapsed() >= timeout {
            return Err(AppError::Internal(format!(
                "Timed out waiting for session {} memory readiness after {} ms",
                thread_id,
                start.elapsed().as_millis()
            )));
        }

        tokio::time::sleep(poll).await;
    }
}

async fn collect_close_wait_status(
    state: &Arc<AppState>,
    thread_id: &str,
    user_id: &str,
    agent_id: &str,
    start: tokio::time::Instant,
) -> Result<CloseAndWaitResponse> {
    let tenant_root = state.current_tenant_root.read().await.clone();
    let root = tenant_root.unwrap_or_else(|| state.data_dir.clone());

    let user_index_path = root.join("user").join(user_id).join(".memory_index.json");
    let timeline_dir = root.join("session").join(thread_id).join("timeline");
    let timeline_abstract = timeline_dir.join(".abstract.md");
    let timeline_overview = timeline_dir.join(".overview.md");

    let session_status = {
        let session_mgr = state.current_session_manager().await;
        let session_mgr = session_mgr.read().await;
        match session_mgr.load_session(thread_id).await {
            Ok(meta) => format!("{:?}", meta.status),
            Err(_) => "Unknown".to_string(),
        }
    };

    let user_index = read_memory_index(&user_index_path)?;
    let user_index_exists = user_index.is_some();
    let user_memory_count = user_index.as_ref().map(|idx| idx.memories.len()).unwrap_or(0);

    let session_summary = user_index
        .as_ref()
        .and_then(|idx| idx.session_summaries.get(thread_id));
    let session_summary_exists = session_summary.is_some();
    let session_summary_memory_count = session_summary
        .map(|s| s.memories_created.len() + s.memories_updated.len())
        .unwrap_or(0);

    if let (Some(index), Some(summary)) = (user_index.as_ref(), session_summary) {
        ensure_session_memory_vectors(state, user_id, index, summary).await?;
    }

    let vector_store = state.vector_store.read().await.clone();
    let vector_sync_confirmed = if let (Some(index), Some(summary), Some(store)) = (
        user_index.as_ref(),
        session_summary,
        vector_store.as_ref(),
    ) {
        let ids: Vec<&String> = summary
            .memories_created
            .iter()
            .chain(summary.memories_updated.iter())
            .collect();

        if ids.is_empty() {
            false
        } else {
            let mut all_present = true;
            for memory_id in ids {
                let Some(meta) = index.memories.get(memory_id) else {
                    all_present = false;
                    break;
                };
                let file_uri = format!("cortex://user/{}/{}", user_id, meta.file);
                let vector_id = uri_to_vector_id(&file_uri, ContextLayer::L2Detail);
                if store.get(&vector_id).await?.is_none() {
                    all_present = false;
                    break;
                }
            }
            all_present
        }
    } else {
        false
    };

    Ok(CloseAndWaitResponse {
        thread_id: thread_id.to_string(),
        status: session_status,
        user_id: user_id.to_string(),
        agent_id: agent_id.to_string(),
        waited_ms: start.elapsed().as_millis() as u64,
        user_index_exists,
        user_memory_count,
        session_summary_exists,
        session_summary_memory_count,
        vector_sync_confirmed,
        timeline_abstract_exists: timeline_abstract.exists(),
        timeline_overview_exists: timeline_overview.exists(),
    })
}

fn is_close_wait_ready(status: &CloseAndWaitResponse) -> bool {
    status.status.eq_ignore_ascii_case("Closed")
        && status.user_index_exists
        && status.session_summary_exists
        && status.session_summary_memory_count > 0
        && status.vector_sync_confirmed
}

fn read_memory_index(path: &PathBuf) -> Result<Option<MemoryIndex>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| AppError::Internal(format!("failed to read {}: {}", path.display(), e)))?;
    let index = serde_json::from_str::<MemoryIndex>(&content)
        .map_err(|e| AppError::Internal(format!("failed to parse {}: {}", path.display(), e)))?;
    Ok(Some(index))
}

async fn ensure_session_memory_vectors(
    state: &Arc<AppState>,
    user_id: &str,
    index: &MemoryIndex,
    summary: &SessionExtractionSummary,
) -> Result<()> {
    let memory_ids: Vec<&String> = summary
        .memories_created
        .iter()
        .chain(summary.memories_updated.iter())
        .collect();

    if memory_ids.is_empty() {
        return Ok(());
    }

    let cortex = state.cortex.read().await.clone();
    let Some(embedding) = cortex.embedding() else {
        return Ok(());
    };
    let Some(qdrant) = cortex.qdrant_store() else {
        return Ok(());
    };
    let filesystem = cortex.filesystem();
    let sync = VectorSyncManager::new(filesystem, embedding, qdrant);

    for memory_id in memory_ids {
        let Some(meta) = index.memories.get(memory_id) else {
            continue;
        };
        let file_uri = format!("cortex://user/{}/{}", user_id, meta.file);
        let _ = sync.sync_file_change(&file_uri, ChangeType::Add).await?;
    }

    Ok(())
}
