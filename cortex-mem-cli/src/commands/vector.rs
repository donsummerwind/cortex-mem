use anyhow::Result;
use colored::Colorize;
use cortex_mem_tools::MemoryOperations;
use std::sync::Arc;

fn qdrant_url() -> String {
    std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string())
}

/// Reindex: clean up no-URI vectors, then full sync
pub async fn reindex(operations: Arc<MemoryOperations>) -> Result<()> {
    println!("{} Starting vector reindex...\n", "🔄".bold());

    // Step 1: delete stale (no-URI) vectors
    match clean_no_uri_vectors().await {
        Ok(n) => println!("  {} Removed {} stale vectors (no URI metadata)", "✅".green(), n),
        Err(e) => println!("  {} Failed to clean stale vectors: {} (continuing...)", "⚠️".yellow(), e),
    }

    // Step 2: full sync
    println!("\n{} Syncing all files to vector database...", "📦".bold());
    let stats = operations.index_all_files().await?;

    println!("\n{} Reindex complete!", "✅".bold());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Statistics:");
    println!("   • Files processed:       {}", stats.total_files);
    println!("   • Newly indexed:         {}", stats.indexed_files);
    println!("   • Skipped (up-to-date):  {}", stats.skipped_files);
    println!("   • Errors:                {}", stats.error_files);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if stats.error_files > 0 {
        println!("\n⚠️  Some files failed to index. Run with --verbose for details.");
    }

    Ok(())
}

/// Prune: delete vectors whose corresponding files no longer exist on disk
pub async fn prune(operations: Arc<MemoryOperations>, dry_run: bool) -> Result<()> {
    if dry_run {
        println!("{} Scanning for dangling vectors (dry-run, no changes)...\n", "🔍".bold());
    } else {
        println!("{} Scanning for dangling vectors (files deleted from disk)...\n", "🧹".bold());
    }

    let url = qdrant_url();
    let client = reqwest::Client::new();

    let resp: serde_json::Value = client.get(format!("{}/collections", url))
        .send().await?.json().await?;
    let collections: Vec<String> = resp["result"]["collections"]
        .as_array().unwrap_or(&vec![])
        .iter()
        .filter_map(|c| c["name"].as_str().map(|s| s.to_string()))
        .collect();

    let mut total_checked = 0u64;
    let mut dangling_ids: Vec<(String, serde_json::Value)> = vec![]; // (collection, point_id)

    for coll in &collections {
        let mut offset: Option<serde_json::Value> = None;

        loop {
            let mut body = serde_json::json!({
                "limit": 200,
                "with_payload": ["uri"],
                "with_vector": false,
                "filter": {
                    "must_not": [{"is_empty": {"key": "uri"}}]
                }
            });
            if let Some(ref off) = offset {
                body["offset"] = off.clone();
            }

            let scroll: serde_json::Value = client
                .post(format!("{}/collections/{}/points/scroll", url, coll))
                .json(&body)
                .send().await?.json().await?;

            let points = match scroll["result"]["points"].as_array() {
                Some(p) if !p.is_empty() => p.clone(),
                _ => break,
            };

            for pt in &points {
                total_checked += 1;
                let uri = match pt["payload"]["uri"].as_str() {
                    Some(u) if !u.is_empty() => u,
                    _ => continue,
                };

                // Check if the file still exists in cortex filesystem
                let exists = operations.exists(uri).await.unwrap_or(true); // assume exists on error
                if !exists {
                    if dry_run {
                        println!("  {} would delete: {}", "→".dimmed(), uri);
                    }
                    dangling_ids.push((coll.clone(), pt["id"].clone()));
                }
            }

            offset = scroll["result"]["next_page_offset"].clone().into();
            if offset.as_ref().map(|v| v.is_null()).unwrap_or(true) {
                break;
            }
        }
    }

    println!("\n  Checked {} vectors", total_checked);
    println!("  Dangling (file missing): {}", dangling_ids.len());

    if dangling_ids.is_empty() {
        println!("\n{} No dangling vectors found.", "✅".green());
        return Ok(());
    }

    if dry_run {
        println!(
            "\n{} Dry-run complete. Run without --dry-run to actually delete {} vectors.",
            "ℹ️".cyan(), dangling_ids.len()
        );
        return Ok(());
    }

    // Group by collection and batch-delete
    let mut by_coll: std::collections::HashMap<String, Vec<serde_json::Value>> =
        std::collections::HashMap::new();
    for (coll, id) in dangling_ids {
        by_coll.entry(coll).or_default().push(id);
    }

    let mut total_deleted = 0usize;
    for (coll, ids) in &by_coll {
        let del: serde_json::Value = client
            .post(format!("{}/collections/{}/points/delete", url, coll))
            .json(&serde_json::json!({"points": ids}))
            .send().await?.json().await?;
        if del["status"].as_str() == Some("ok") {
            total_deleted += ids.len();
        }
    }

    println!("\n{} Pruned {} dangling vectors.", "✅".green(), total_deleted);
    Ok(())
}

/// Show vector index status for the current tenant
pub async fn status(operations: Arc<MemoryOperations>) -> Result<()> {
    println!("{} Vector index status\n", "📊".bold());

    // Count total tracked files
    let mut total_files = 0usize;
    for root in &["cortex://session", "cortex://user", "cortex://agent"] {
        if let Ok(files) = operations.list_files(root).await {
            total_files += files.len();
        }
    }
    println!("  Total tracked files: ~{}", total_files);

    match fetch_collection_stats().await {
        Ok((total_pts, no_uri_pts)) => {
            println!("  Vectors in Qdrant:   {}", total_pts);
            if no_uri_pts > 0 {
                println!(
                    "  Missing URI (stale): {} {}",
                    no_uri_pts,
                    "(run `vector reindex` to fix)".yellow()
                );
            } else {
                println!("  Missing URI (stale): 0 ✅");
            }
        }
        Err(e) => {
            println!("  {} Could not reach Qdrant: {}", "⚠️".yellow(), e);
        }
    }

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

async fn fetch_collection_stats() -> Result<(u64, u64)> {
    let url = qdrant_url();
    let client = reqwest::Client::new();

    let resp: serde_json::Value = client.get(format!("{}/collections", url))
        .send().await?.json().await?;

    let collections: Vec<String> = resp["result"]["collections"]
        .as_array().unwrap_or(&vec![])
        .iter()
        .filter_map(|c| c["name"].as_str().map(|s| s.to_string()))
        .collect();

    if collections.is_empty() {
        anyhow::bail!("No Qdrant collections found");
    }

    let mut total_pts = 0u64;
    let mut no_uri_pts = 0u64;

    for coll in &collections {
        let info: serde_json::Value = client
            .get(format!("{}/collections/{}", url, coll))
            .send().await?.json().await?;
        total_pts += info["result"]["points_count"].as_u64().unwrap_or(0);

        let count_resp: serde_json::Value = client
            .post(format!("{}/collections/{}/points/count", url, coll))
            .json(&serde_json::json!({
                "filter": {
                    "must": [{"is_empty": {"key": "uri"}}]
                }
            }))
            .send().await?.json().await?;

        no_uri_pts += count_resp["result"]["count"].as_u64().unwrap_or(0);
    }

    Ok((total_pts, no_uri_pts))
}

async fn clean_no_uri_vectors() -> Result<usize> {
    let url = qdrant_url();
    let client = reqwest::Client::new();

    let resp: serde_json::Value = client.get(format!("{}/collections", url))
        .send().await?.json().await?;

    let collections: Vec<String> = resp["result"]["collections"]
        .as_array().unwrap_or(&vec![])
        .iter()
        .filter_map(|c| c["name"].as_str().map(|s| s.to_string()))
        .collect();

    let mut total_deleted = 0usize;

    for coll in &collections {
        let count_resp: serde_json::Value = client
            .post(format!("{}/collections/{}/points/count", url, coll))
            .json(&serde_json::json!({
                "filter": {
                    "must": [{"is_empty": {"key": "uri"}}]
                }
            }))
            .send().await?.json().await?;

        let count = count_resp["result"]["count"].as_u64().unwrap_or(0);
        if count == 0 {
            continue;
        }

        let del_resp: serde_json::Value = client
            .post(format!("{}/collections/{}/points/delete", url, coll))
            .json(&serde_json::json!({
                "filter": {
                    "must": [{"is_empty": {"key": "uri"}}]
                }
            }))
            .send().await?.json().await?;

        if del_resp["status"].as_str() == Some("ok") {
            total_deleted += count as usize;
        }
    }

    Ok(total_deleted)
}
