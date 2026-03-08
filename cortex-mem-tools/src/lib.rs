pub mod errors;
pub mod mcp;
pub mod operations;
pub mod tools;
pub mod types;

pub use errors::{Result, ToolsError};
pub use mcp::{ToolDefinition, get_mcp_tool_definition, get_mcp_tool_definitions};
pub use operations::MemoryOperations;
pub use types::*;

// 重新导出长期运行服务 API 相关类型
pub use operations::{PendingStatus, ProcessingResult};

pub use cortex_mem_core::automation::GenerationStats;

// 重新导出 SyncStats 以便外部使用
pub use cortex_mem_core::automation::SyncStats;
