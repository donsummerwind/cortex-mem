// MCP Tool Definitions
//
// Unified tool definitions used by both cortex-mem-mcp and cortex-mem-service.
// This ensures consistent naming and parameters across all integration points.
//
// Tool Naming Convention: Simple verb style (search, store, ls, etc.)
//
// Layer System:
// - L0: Abstract (~100 tokens) - for quick relevance checking
// - L1: Overview (~2000 tokens) - for understanding core information
// - L2: Full content - complete original content

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Get all MCP tool definitions
pub fn get_mcp_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        // ==================== Search Tools ====================
        ToolDefinition {
            name: "search".to_string(),
            description: include_str!("../docs/search.md").to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query - can be natural language or keywords"
                    },
                    "scope": {
                        "type": "string",
                        "description": "Optional session/thread ID to limit search scope, or 'user', 'agent', 'session'"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results to return (default: 10)",
                        "default": 10
                    },
                    "min_score": {
                        "type": "number",
                        "description": "Minimum relevance score threshold (0-1, default: 0.5)",
                        "default": 0.5
                    },
                    "return_layers": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["L0", "L1", "L2"]
                        },
                        "description": "Which layers to return. Default: [\"L0\"]. Use [\"L0\",\"L1\"] for more context, [\"L0\",\"L1\",\"L2\"] for full content.",
                        "default": ["L0"]
                    }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "recall".to_string(),
            description: include_str!("../docs/recall.md").to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    },
                    "scope": {
                        "type": "string",
                        "description": "Optional session/thread ID to limit search scope"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results (default: 10)",
                        "default": 10
                    }
                },
                "required": ["query"]
            }),
        },
        // ==================== Storage Tools ====================
        ToolDefinition {
            name: "store".to_string(),
            description: include_str!("../docs/store.md").to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The content to store in memory"
                    },
                    "thread_id": {
                        "type": "string",
                        "description": "Thread/session ID (uses default if not specified)"
                    },
                    "role": {
                        "type": "string",
                        "enum": ["user", "assistant", "system"],
                        "description": "Role of the message sender (default: user)",
                        "default": "user"
                    }
                },
                "required": ["content"]
            }),
        },
        ToolDefinition {
            name: "commit".to_string(),
            description: include_str!("../docs/commit.md").to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "thread_id": {
                        "type": "string",
                        "description": "Thread/session ID to commit (uses default if not specified)"
                    }
                }
            }),
        },
        // ==================== Filesystem Tools ====================
        ToolDefinition {
            name: "ls".to_string(),
            description: include_str!("../docs/ls.md").to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uri": {
                        "type": "string",
                        "description": "Directory URI to list (default: cortex://session)",
                        "default": "cortex://session"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "Whether to recursively list subdirectories",
                        "default": false
                    },
                    "include_abstracts": {
                        "type": "boolean",
                        "description": "Whether to include L0 abstracts for each file",
                        "default": false
                    }
                }
            }),
        },
        // ==================== Tiered Access Tools ====================
        ToolDefinition {
            name: "abstract".to_string(),
            description: "Get L0 abstract layer (~100 tokens) for quick relevance checking.\n\nAbstracts are short summaries ideal for quickly determining if content is relevant before committing to reading more. Use this to minimize token consumption.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uri": {
                        "type": "string",
                        "description": "Content URI (file or directory)"
                    }
                },
                "required": ["uri"]
            }),
        },
        ToolDefinition {
            name: "overview".to_string(),
            description: "Get L1 overview layer (~2000 tokens) with core information and context.\n\nOverviews contain key points and contextual information. Use this when the abstract was relevant but you need more details.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uri": {
                        "type": "string",
                        "description": "Content URI (file or directory)"
                    }
                },
                "required": ["uri"]
            }),
        },
        ToolDefinition {
            name: "content".to_string(),
            description: "Get L2 full content layer - the complete original content.\n\nUse this ONLY when you need the complete, unprocessed content. This returns the full content which may be large.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uri": {
                        "type": "string",
                        "description": "Content URI (file only)"
                    }
                },
                "required": ["uri"]
            }),
        },
        // ==================== Exploration Tool ====================
        ToolDefinition {
            name: "explore".to_string(),
            description: include_str!("../docs/explore.md").to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Exploration query - what to look for"
                    },
                    "start_uri": {
                        "type": "string",
                        "description": "Starting URI for exploration",
                        "default": "cortex://session"
                    },
                    "return_layers": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["L0", "L1", "L2"]
                        },
                        "description": "Which layers to return in matches",
                        "default": ["L0"]
                    }
                },
                "required": ["query"]
            }),
        },
        // ==================== Management Tools ====================
        ToolDefinition {
            name: "delete".to_string(),
            description: "Delete a memory by its URI.\n\nThis removes the memory from both the filesystem and the vector database (all layers: L0, L1, L2).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uri": {
                        "type": "string",
                        "description": "URI of the memory to delete"
                    }
                },
                "required": ["uri"]
            }),
        },
        ToolDefinition {
            name: "layers".to_string(),
            description: "Generate L0/L1 layer files for memories.\n\nThis command generates .abstract.md (L0) and .overview.md (L1) files for directories that are missing them.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "thread_id": {
                        "type": "string",
                        "description": "Thread/session ID (optional, if not provided, generates for all sessions)"
                    }
                }
            }),
        },
        ToolDefinition {
            name: "index".to_string(),
            description: "Index memories to vector database.\n\nThis command syncs all memory files to the vector database for semantic search.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "thread_id": {
                        "type": "string",
                        "description": "Thread/session ID (optional, if not provided, indexes all files)"
                    }
                }
            }),
        },
    ]
}

/// Get a specific tool definition by name
pub fn get_mcp_tool_definition(name: &str) -> Option<ToolDefinition> {
    get_mcp_tool_definitions()
        .into_iter()
        .find(|def| def.name == name)
}