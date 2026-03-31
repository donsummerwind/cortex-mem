use cortex_mem_tools::MemoryOperations;
use cortex_mem_tools::types::{ExploreArgs, LsArgs, SearchArgs};
use rmcp::{
    handler::server::tool::ToolRouter, handler::server::wrapper::Parameters, model::*,
    tool, tool_handler, tool_router, Json, ServerHandler,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

// ==================== Auto-Trigger Configuration ====================

/// Configuration for automatic processing triggers
///
/// This enables MCP clients (like Zed) that don't actively call close_session
/// to still have user/agent memories extracted and indexed automatically.
#[derive(Debug, Clone, Copy)]
pub struct AutoTriggerConfig {
    /// Minimum message count before triggering processing
    pub message_count_threshold: usize,
    /// Minimum time interval between processing (in seconds)
    pub min_process_interval_secs: u64,
    /// Inactivity timeout to trigger processing (in seconds)
    /// If no new messages for this duration, trigger processing
    pub inactivity_timeout_secs: u64,
    /// Enable auto-trigger on store_memory
    pub enable_auto_trigger: bool,
}

impl Default for AutoTriggerConfig {
    fn default() -> Self {
        Self {
            message_count_threshold: 10,       // Trigger after 10 messages
            min_process_interval_secs: 300,    // At most once every 5 minutes
            inactivity_timeout_secs: 120,      // Trigger after 2 min of inactivity
            enable_auto_trigger: true,
        }
    }
}

/// Session state for auto-trigger tracking
#[derive(Debug)]
struct SessionState {
    /// Number of messages since last processing
    message_count: usize,
    /// Time of last processing
    last_processed: Option<Instant>,
    /// Time of last message
    last_message: Instant,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            message_count: 0,
            last_processed: None,
            last_message: Instant::now(),
        }
    }
}

// ==================== Tool Arguments & Results ====================

// Store Tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StoreArgs {
    /// Content to store
    pub content: String,
    /// Thread/session ID (optional, defaults to "default")
    pub thread_id: Option<String>,
    /// Message role: "user", "assistant", or "system"
    pub role: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StoreResult {
    pub success: bool,
    pub uri: String,
    pub message_id: String,
}

// Search Tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchArgsMcp {
    /// Search query
    pub query: String,
    /// Thread ID to search in (optional)
    pub scope: Option<String>,
    /// Maximum number of results (default: 10)
    pub limit: Option<usize>,
    /// Minimum relevance score (0-1, default: 0.5)
    pub min_score: Option<f32>,
    /// Which layers to return: ["L0"] (default), ["L0","L1"], ["L0","L1","L2"]
    pub return_layers: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchResultMcp {
    pub uri: String,
    pub score: f32,
    pub snippet: String,
    pub overview: Option<String>,
    pub content: Option<String>,
    pub layers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchResultMcpList {
    pub success: bool,
    pub query: String,
    pub results: Vec<SearchResultMcp>,
    pub total: usize,
}

// Recall Tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RecallArgs {
    /// The search query
    pub query: String,
    /// Optional session/thread ID to limit search scope
    pub scope: Option<String>,
    /// Maximum number of results (default: 10)
    pub limit: Option<usize>,
}

// Ls Tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LsArgsMcp {
    /// URI to list (e.g., "cortex://session" or "cortex://user/preferences")
    pub uri: Option<String>,
    /// Whether to recursively list subdirectories
    pub recursive: Option<bool>,
    /// Include abstracts in results
    pub include_abstracts: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LsEntryMcp {
    pub name: String,
    pub uri: String,
    pub is_directory: bool,
    pub size: Option<usize>,
    pub abstract_text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LsResult {
    pub success: bool,
    pub uri: String,
    pub entries: Vec<LsEntryMcp>,
    pub total: usize,
}

// Explore Tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ExploreArgsMcp {
    /// Exploration query - what to look for
    pub query: String,
    /// Starting URI for exploration
    pub start_uri: Option<String>,
    /// Which layers to return in matches
    pub return_layers: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ExplorationPathItemMcp {
    pub uri: String,
    pub relevance_score: f32,
    pub abstract_text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ExploreResult {
    pub success: bool,
    pub query: String,
    pub exploration_path: Vec<ExplorationPathItemMcp>,
    pub matches: Vec<SearchResultMcp>,
    pub total_explored: usize,
    pub total_matches: usize,
}

// Tiered Access Tools
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AbstractArgs {
    /// URI of the memory
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AbstractResult {
    pub success: bool,
    pub uri: String,
    pub abstract_text: String,
    pub layer: String,
    pub token_count: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct OverviewArgs {
    /// URI of the memory
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct OverviewResult {
    pub success: bool,
    pub uri: String,
    pub overview_text: String,
    pub layer: String,
    pub token_count: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ContentArgs {
    /// URI of the memory to retrieve
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ContentResult {
    pub success: bool,
    pub uri: String,
    pub content: String,
    pub layer: String,
    pub token_count: usize,
}

// Delete Tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DeleteArgs {
    /// URI of the memory to delete
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DeleteResult {
    pub success: bool,
    pub uri: String,
}

// Commit Tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CommitArgs {
    /// Thread/session ID to commit
    pub thread_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CommitResult {
    pub success: bool,
    pub thread_id: String,
    pub message: String,
}

// Layers Tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LayersArgs {
    /// Thread/session ID (optional, if not provided, generates for all sessions)
    pub thread_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LayersResult {
    pub success: bool,
    pub message: String,
    pub total: usize,
    pub generated: usize,
    pub failed: usize,
}

// Index Tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IndexArgs {
    /// Thread/session ID (optional, if not provided, indexes all files)
    pub thread_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IndexResult {
    pub success: bool,
    pub message: String,
    pub total_files: usize,
    pub indexed_files: usize,
    pub skipped_files: usize,
    pub error_files: usize,
}

// ==================== MCP Service ====================

/// MCP Service for Cortex Memory
///
/// Provides automatic triggering of memory extraction and layer generation
/// to support MCP clients that don't actively call close_session.
///
/// ## Auto-Trigger Mechanism
///
/// When `store` is called, the service checks:
/// 1. Message count threshold (default: 10 messages)
/// 2. Inactivity timeout (default: 2 minutes without new messages)
///
/// If conditions are met, it sends a `SessionClosed` event to the
/// `MemoryEventCoordinator`, which handles:
/// - Memory extraction (session → user/agent memories)
/// - L0/L1 layer generation
/// - Vector indexing
#[derive(Clone)]
pub struct MemoryMcpService {
    operations: Arc<MemoryOperations>,
    tool_router: ToolRouter<Self>,
    /// Auto-trigger configuration
    auto_trigger_config: AutoTriggerConfig,
    /// Session states for tracking auto-trigger conditions
    /// Key: thread_id, Value: session state
    session_states: Arc<RwLock<std::collections::HashMap<String, SessionState>>>,
    /// Last global processing time (to prevent too frequent processing)
    last_global_process: Arc<AtomicU64>,
}

#[tool_router]
impl MemoryMcpService {
    /// Create a new MCP service with auto-trigger configuration
    pub fn with_config(operations: Arc<MemoryOperations>, config: AutoTriggerConfig) -> Self {
        Self {
            operations,
            tool_router: Self::tool_router(),
            auto_trigger_config: config,
            session_states: Arc::new(RwLock::new(std::collections::HashMap::new())),
            last_global_process: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Check if auto-trigger conditions are met and send SessionClosed event
    async fn check_and_trigger_processing(&self, thread_id: &str) -> bool {
        if !self.auto_trigger_config.enable_auto_trigger {
            return false;
        }

        let mut states = self.session_states.write().await;
        let state = states.entry(thread_id.to_string()).or_default();

        // Update last message time
        state.last_message = Instant::now();
        state.message_count += 1;

        let should_trigger = self.should_trigger_processing_inner(state);

        if should_trigger {
            // Reset state
            state.message_count = 0;
            state.last_processed = Some(Instant::now());

            // Update global processing time (Unix timestamp in seconds)
            let now_ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            self.last_global_process.store(now_ts, Ordering::Relaxed);

            // Send SessionClosed event to MemoryEventCoordinator
            if let Some(tx) = self.operations.memory_event_tx() {
                use cortex_mem_core::memory_events::MemoryEvent;

                let user_id = self.operations.default_user_id().to_string();
                let agent_id = self.operations.default_agent_id().to_string();

                let _ = tx.send(MemoryEvent::SessionClosed {
                    session_id: thread_id.to_string(),
                    user_id,
                    agent_id,
                });

                info!(
                    "Auto-triggered SessionClosed event for session {} (will process in background)",
                    thread_id
                );
            } else {
                warn!("memory_event_tx not available, cannot auto-trigger processing");
            }

            return true;
        }

        false
    }

    /// Check if processing should be triggered based on session state
    fn should_trigger_processing_inner(&self, state: &SessionState) -> bool {
        let config = &self.auto_trigger_config;

        // Condition 1: Message count threshold
        if state.message_count >= config.message_count_threshold {
            // Check minimum interval since last processing
            if let Some(last_processed) = state.last_processed {
                let elapsed = last_processed.elapsed().as_secs();
                if elapsed < config.min_process_interval_secs {
                    debug!(
                        "Message threshold reached but min interval not met ({}s < {}s)",
                        elapsed, config.min_process_interval_secs
                    );
                    return false;
                }
            }
            info!(
                "Auto-trigger: message count {} >= threshold {}",
                state.message_count, config.message_count_threshold
            );
            return true;
        }

        false
    }

    /// Start a background task to check for inactive sessions
    pub fn start_inactivity_checker(&self) {
        let session_states = self.session_states.clone();
        let operations = self.operations.clone();
        let config = self.auto_trigger_config;

        tokio::spawn(async move {
            let check_interval = std::time::Duration::from_secs(30);
            let mut interval = tokio::time::interval(check_interval);

            loop {
                interval.tick().await;

                if !config.enable_auto_trigger {
                    continue;
                }

                let mut states = session_states.write().await;
                let mut to_process = Vec::new();

                for (thread_id, state) in states.iter_mut() {
                    let inactive_duration = state.last_message.elapsed().as_secs();

                    if inactive_duration >= config.inactivity_timeout_secs && state.message_count > 0
                    {
                        let can_process = if let Some(last_processed) = state.last_processed {
                            last_processed.elapsed().as_secs() >= config.min_process_interval_secs
                        } else {
                            true
                        };

                        if can_process {
                            info!(
                                "Session {} inactive for {}s, triggering processing",
                                thread_id, inactive_duration
                            );
                            to_process.push(thread_id.clone());
                        }
                    }
                }

                for thread_id in to_process {
                    if let Some(state) = states.get_mut(&thread_id) {
                        state.message_count = 0;
                        state.last_processed = Some(Instant::now());

                        if let Some(tx) = operations.memory_event_tx() {
                            use cortex_mem_core::memory_events::MemoryEvent;

                            let user_id = operations.default_user_id().to_string();
                            let agent_id = operations.default_agent_id().to_string();

                            let _ = tx.send(MemoryEvent::SessionClosed {
                                session_id: thread_id.clone(),
                                user_id,
                                agent_id,
                            });
                        }
                    }
                }
            }
        });

        info!("Session inactivity checker started");
    }

    // ==================== Search Tools ====================

    #[tool(description = "Layered semantic search across memory using L0/L1/L2 tiered retrieval")]
    async fn search(
        &self,
        params: Parameters<SearchArgsMcp>,
    ) -> std::result::Result<Json<SearchResultMcpList>, String> {
        debug!("search called with args: {:?}", params.0);

        let limit = params.0.limit.unwrap_or(10);
        let _min_score = params.0.min_score.unwrap_or(0.5);
        let return_layers = params.0.return_layers.unwrap_or_else(|| vec!["L0".to_string()]);

        let search_args = SearchArgs {
            query: params.0.query.clone(),
            recursive: Some(true),
            return_layers: Some(return_layers.clone()),
            scope: params.0.scope.clone(),
            limit: Some(limit),
        };

        match self.operations.search(search_args).await {
            Ok(response) => {
                let results: Vec<SearchResultMcp> = response
                    .results
                    .into_iter()
                    .map(|r| SearchResultMcp {
                        uri: r.uri,
                        score: r.score,
                        snippet: r.abstract_text.clone().unwrap_or_default(),
                        overview: r.overview_text,
                        content: r.content,
                        layers: return_layers.clone(),
                    })
                    .collect();

                let total = results.len();
                info!("Search '{}' found {} results", params.0.query, total);

                Ok(Json(SearchResultMcpList {
                    success: true,
                    query: params.0.query,
                    results,
                    total,
                }))
            }
            Err(e) => {
                error!("Search failed: {}", e);
                Err(format!("Search failed: {}", e))
            }
        }
    }

    #[tool(description = "Recall memories with full context (L0 snippet + L2 content)")]
    async fn recall(
        &self,
        params: Parameters<RecallArgs>,
    ) -> std::result::Result<Json<SearchResultMcpList>, String> {
        debug!("recall called with args: {:?}", params.0);

        match self
            .operations
            .recall(&params.0.query, params.0.scope.as_deref(), params.0.limit)
            .await
        {
            Ok(response) => {
                let results: Vec<SearchResultMcp> = response
                    .results
                    .into_iter()
                    .map(|r| SearchResultMcp {
                        uri: r.uri,
                        score: r.score,
                        snippet: r.abstract_text.clone().unwrap_or_default(),
                        overview: r.overview_text,
                        content: r.content,
                        layers: vec!["L0".to_string(), "L2".to_string()],
                    })
                    .collect();

                let total = results.len();
                info!("Recall '{}' found {} results", params.0.query, total);

                Ok(Json(SearchResultMcpList {
                    success: true,
                    query: params.0.query,
                    results,
                    total,
                }))
            }
            Err(e) => {
                error!("Recall failed: {}", e);
                Err(format!("Recall failed: {}", e))
            }
        }
    }

    // ==================== Storage Tools ====================

    #[tool(description = "Add a message to memory for a specific session")]
    async fn store(
        &self,
        params: Parameters<StoreArgs>,
    ) -> std::result::Result<Json<StoreResult>, String> {
        debug!("store called with args: {:?}", params.0);

        let thread_id = params.0.thread_id.unwrap_or_else(|| "default".to_string());
        let role = params.0.role.as_deref().unwrap_or("user");

        match self
            .operations
            .add_message(&thread_id, role, &params.0.content)
            .await
        {
            Ok(message_uri) => {
                let message_id = message_uri
                    .rsplit('/')
                    .next()
                    .and_then(|s| s.strip_suffix(".md"))
                    .unwrap_or("unknown")
                    .to_string();

                info!("Memory stored at: {}", message_uri);

                let triggered = self.check_and_trigger_processing(&thread_id).await;
                if triggered {
                    info!("Auto-triggered memory processing for thread {}", thread_id);
                }

                Ok(Json(StoreResult {
                    success: true,
                    uri: message_uri,
                    message_id,
                }))
            }
            Err(e) => {
                error!("Failed to store memory: {}", e);
                Err(format!("Failed to store memory: {}", e))
            }
        }
    }

    #[tool(description = "Commit accumulated conversation content and trigger memory extraction")]
    async fn commit(
        &self,
        params: Parameters<CommitArgs>,
    ) -> std::result::Result<Json<CommitResult>, String> {
        debug!("commit called with args: {:?}", params.0);

        let thread_id = params.0.thread_id.unwrap_or_else(|| "default".to_string());

        match self.operations.close_session_sync(&thread_id).await {
            Ok(_) => {
                info!("Session {} closed and fully processed (sync)", thread_id);

                Ok(Json(CommitResult {
                    success: true,
                    thread_id: thread_id.clone(),
                    message: "Session committed. All processing (memory extraction, L0/L1 generation, vector sync) completed.".to_string(),
                }))
            }
            Err(e) => {
                error!("Failed to commit session {}: {}", thread_id, e);
                Err(format!("Failed to commit session: {}", e))
            }
        }
    }

    // ==================== Filesystem Tools ====================

    #[tool(description = "List directory contents to browse the memory space")]
    async fn ls(
        &self,
        params: Parameters<LsArgsMcp>,
    ) -> std::result::Result<Json<LsResult>, String> {
        debug!("ls called with args: {:?}", params.0);

        let uri = params.0.uri.as_deref().unwrap_or("cortex://session");
        let include_abstracts = params.0.include_abstracts.unwrap_or(false);

        let ls_args = LsArgs {
            uri: uri.to_string(),
            recursive: params.0.recursive,
            include_abstracts: Some(include_abstracts),
        };

        match self.operations.ls(ls_args).await {
            Ok(response) => {
                let entries: Vec<LsEntryMcp> = response
                    .entries
                    .into_iter()
                    .map(|e| LsEntryMcp {
                        name: e.name,
                        uri: e.uri,
                        is_directory: e.is_directory,
                        size: e.child_count.map(|c| c as usize),
                        abstract_text: e.abstract_text,
                    })
                    .collect();

                let total = entries.len();
                info!("Listed {} items at {}", total, uri);

                Ok(Json(LsResult {
                    success: true,
                    uri: uri.to_string(),
                    entries,
                    total,
                }))
            }
            Err(e) => {
                error!("List failed: {}", e);
                Err(format!("Failed to list: {}", e))
            }
        }
    }

    // ==================== Exploration Tool ====================

    #[tool(description = "Smart exploration of memory space, combining search and browsing")]
    async fn explore(
        &self,
        params: Parameters<ExploreArgsMcp>,
    ) -> std::result::Result<Json<ExploreResult>, String> {
        debug!("explore called with args: {:?}", params.0);

        let explore_args = ExploreArgs {
            query: params.0.query.clone(),
            start_uri: params.0.start_uri.clone(),
            max_depth: Some(3),
            return_layers: params.0.return_layers.clone(),
        };

        match self.operations.explore(explore_args).await {
            Ok(response) => {
                let exploration_path: Vec<ExplorationPathItemMcp> = response
                    .exploration_path
                    .into_iter()
                    .map(|item| ExplorationPathItemMcp {
                        uri: item.uri,
                        relevance_score: item.relevance_score,
                        abstract_text: item.abstract_text,
                    })
                    .collect();

                let matches: Vec<SearchResultMcp> = response
                    .matches
                    .into_iter()
                    .map(|m| SearchResultMcp {
                        uri: m.uri,
                        score: m.score,
                        snippet: m.abstract_text.clone().unwrap_or_default(),
                        overview: m.overview_text,
                        content: m.content,
                        layers: params.0.return_layers.clone().unwrap_or_else(|| vec!["L0".to_string()]),
                    })
                    .collect();

                info!(
                    "Explore '{}' found {} matches from {} explored items",
                    params.0.query, response.total_matches, response.total_explored
                );

                Ok(Json(ExploreResult {
                    success: true,
                    query: params.0.query,
                    exploration_path,
                    matches,
                    total_explored: response.total_explored,
                    total_matches: response.total_matches,
                }))
            }
            Err(e) => {
                error!("Explore failed: {}", e);
                Err(format!("Explore failed: {}", e))
            }
        }
    }

    // ==================== Tiered Access Tools ====================

    #[tool(description = "Get L0 abstract layer (~100 tokens) for quick relevance checking")]
    async fn r#abstract(
        &self,
        params: Parameters<AbstractArgs>,
    ) -> std::result::Result<Json<AbstractResult>, String> {
        debug!("abstract called with args: {:?}", params.0);

        match self.operations.get_abstract(&params.0.uri).await {
            Ok(abstract_result) => {
                info!("Abstract retrieved for: {}", params.0.uri);
                Ok(Json(AbstractResult {
                    success: true,
                    uri: params.0.uri.clone(),
                    abstract_text: abstract_result.abstract_text,
                    layer: abstract_result.layer,
                    token_count: abstract_result.token_count,
                }))
            }
            Err(e) => {
                error!("Failed to get abstract: {}", e);
                Err(format!("Failed to get abstract: {}", e))
            }
        }
    }

    #[tool(description = "Get L1 overview layer (~2000 tokens) for understanding core information")]
    async fn overview(
        &self,
        params: Parameters<OverviewArgs>,
    ) -> std::result::Result<Json<OverviewResult>, String> {
        debug!("overview called with args: {:?}", params.0);

        match self.operations.get_overview(&params.0.uri).await {
            Ok(overview_result) => {
                info!("Overview retrieved for: {}", params.0.uri);
                Ok(Json(OverviewResult {
                    success: true,
                    uri: params.0.uri.clone(),
                    overview_text: overview_result.overview_text,
                    layer: overview_result.layer,
                    token_count: overview_result.token_count,
                }))
            }
            Err(e) => {
                error!("Failed to get overview: {}", e);
                Err(format!("Failed to get overview: {}", e))
            }
        }
    }

    #[tool(description = "Get L2 full content layer - the complete original content")]
    async fn content(
        &self,
        params: Parameters<ContentArgs>,
    ) -> std::result::Result<Json<ContentResult>, String> {
        debug!("content called with args: {:?}", params.0);

        match self.operations.read_file(&params.0.uri).await {
            Ok(content) => {
                let token_count = content.split_whitespace().count();
                info!("Content retrieved from: {}", params.0.uri);
                Ok(Json(ContentResult {
                    success: true,
                    uri: params.0.uri.clone(),
                    content,
                    layer: "L2".to_string(),
                    token_count,
                }))
            }
            Err(e) => {
                error!("Failed to get content: {}", e);
                Err(format!("Failed to get content: {}", e))
            }
        }
    }

    // ==================== Management Tools ====================

    #[tool(description = "Delete a memory by its URI")]
    async fn delete(
        &self,
        params: Parameters<DeleteArgs>,
    ) -> std::result::Result<Json<DeleteResult>, String> {
        debug!("delete called with args: {:?}", params.0);

        match self.operations.delete(&params.0.uri).await {
            Ok(_) => {
                info!("Memory deleted: {}", params.0.uri);
                Ok(Json(DeleteResult {
                    success: true,
                    uri: params.0.uri.clone(),
                }))
            }
            Err(e) => {
                error!("Failed to delete memory: {}", e);
                Err(format!("Failed to delete memory: {}", e))
            }
        }
    }

    #[tool(description = "Generate L0/L1 layer files for memories")]
    async fn layers(
        &self,
        params: Parameters<LayersArgs>,
    ) -> std::result::Result<Json<LayersResult>, String> {
        debug!("layers called with args: {:?}", params.0);

        let (stats, message) = if let Some(ref thread_id) = params.0.thread_id {
            match self.operations.ensure_session_layers(thread_id).await {
                Ok(stats) => {
                    let msg = format!("Generated layers for session {}", thread_id);
                    (stats, msg)
                }
                Err(e) => {
                    error!("Failed to generate layers for session {}: {}", thread_id, e);
                    return Err(format!("Failed to generate layers: {}", e));
                }
            }
        } else {
            match self.operations.ensure_all_layers().await {
                Ok(stats) => {
                    let msg = "Generated layers for all sessions".to_string();
                    (stats, msg)
                }
                Err(e) => {
                    error!("Failed to generate layers: {}", e);
                    return Err(format!("Failed to generate layers: {}", e));
                }
            }
        };

        info!(
            "{}: total={}, generated={}, failed={}",
            message, stats.total, stats.generated, stats.failed
        );

        Ok(Json(LayersResult {
            success: true,
            message,
            total: stats.total,
            generated: stats.generated,
            failed: stats.failed,
        }))
    }

    #[tool(description = "Index memories to vector database")]
    async fn index(
        &self,
        params: Parameters<IndexArgs>,
    ) -> std::result::Result<Json<IndexResult>, String> {
        debug!("index called with args: {:?}", params.0);

        let (stats, message) = if let Some(ref thread_id) = params.0.thread_id {
            match self.operations.index_session_files(thread_id).await {
                Ok(stats) => {
                    let msg = format!("Indexed memories for session {}", thread_id);
                    (stats, msg)
                }
                Err(e) => {
                    error!("Failed to index session {}: {}", thread_id, e);
                    return Err(format!("Failed to index memories: {}", e));
                }
            }
        } else {
            match self.operations.index_all_files().await {
                Ok(stats) => {
                    let msg = "Indexed all memory files".to_string();
                    (stats, msg)
                }
                Err(e) => {
                    error!("Failed to index memories: {}", e);
                    return Err(format!("Failed to index memories: {}", e));
                }
            }
        };

        info!(
            "{}: total={}, indexed={}, skipped={}, errors={}",
            message, stats.total_files, stats.indexed_files, stats.skipped_files, stats.error_files
        );

        Ok(Json(IndexResult {
            success: true,
            message,
            total_files: stats.total_files,
            indexed_files: stats.indexed_files,
            skipped_files: stats.skipped_files,
            error_files: stats.error_files,
        }))
    }
}

#[tool_handler]
impl ServerHandler for MemoryMcpService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Cortex Memory MCP Server - Unified memory management tools.\n\n**Tool Naming Convention:** Simple verb style (search, store, ls, etc.)\n\n**Layer System:**\n- L0: Abstract (~100 tokens) - for quick relevance checking\n- L1: Overview (~2000 tokens) - for understanding core information\n- L2: Full content - complete original content\n\n**Automatic Processing:**\nThe server automatically triggers memory extraction and layer generation when:\n- Message count reaches threshold (default: 10 messages)\n- Session becomes inactive (default: 2 minutes without new messages)\n\n**Available tools:**\n- search: Layered semantic search with return_layers support\n- recall: Quick recall with L0+L2 content\n- store: Add a message to memory\n- commit: Commit session and trigger processing\n- ls: Browse memory filesystem\n- explore: Smart exploration of memory space\n- abstract: Get L0 abstract (~100 tokens)\n- overview: Get L1 overview (~2000 tokens)\n- content: Get L2 full content\n- delete: Delete a memory\n- layers: Generate L0/L1 layer files\n- index: Index memories to vector database\n".to_string(),
            ),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
