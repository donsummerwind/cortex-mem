// Recall Tool - Recall memories with full context

use crate::{MemoryOperations, Result, types::*};

impl MemoryOperations {
    /// Recall memories with full context (L0 snippet + L2 content).
    ///
    /// This is a convenience wrapper that returns both abstract and full content.
    /// Equivalent to search with return_layers=["L0", "L2"].
    pub async fn recall(&self, query: &str, scope: Option<&str>, limit: Option<usize>) -> Result<SearchResponse> {
        let normalized_scope = Self::normalize_scope(scope);

        let search_args = SearchArgs {
            query: query.to_string(),
            recursive: Some(true),
            return_layers: Some(vec!["L0".to_string(), "L2".to_string()]),
            scope: Some(normalized_scope),
            limit,
        };

        self.search(search_args).await
    }
}
