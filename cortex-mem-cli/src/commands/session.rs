use anyhow::Result;
use colored::Colorize;
use cortex_mem_tools::MemoryOperations;
use std::sync::Arc;

pub async fn list(operations: Arc<MemoryOperations>) -> Result<()> {
    println!("{} Listing all sessions", "📋".bold());

    let sessions = operations.list_sessions().await?;

    if sessions.is_empty() {
        println!("\n{} No sessions found", "ℹ".yellow().bold());
        return Ok(());
    }

    println!("\n{} Found {} sessions:", "✓".green().bold(), sessions.len());
    println!();

    for session in sessions {
        println!("• {}", session.thread_id.bright_blue().bold());
        println!("  {}: {}", "Status".dimmed(), session.status);
        println!("  {}: {}", "Created".dimmed(), session.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("  {}: {}", "Updated".dimmed(), session.updated_at.format("%Y-%m-%d %H:%M:%S UTC"));
        println!();
    }

    Ok(())
}

pub async fn create(
    operations: Arc<MemoryOperations>,
    thread: &str,
    title: Option<&str>,
) -> Result<()> {
    println!("{} Creating session: {}", "📝".bold(), thread.cyan());

    // Add a system message to create the session
    let message = if let Some(t) = title {
        format!("Session: {}", t)
    } else {
        "Session created".to_string()
    };
    
    operations.add_message(thread, "system", &message).await?;

    println!("{} Session created successfully", "✓".green().bold());
    println!("  {}: {}", "Thread ID".cyan(), thread);
    if let Some(t) = title {
        println!("  {}: {}", "Title".cyan(), t);
    }

    Ok(())
}

/// Close a session and trigger memory extraction, layer generation, and indexing
pub async fn close(operations: Arc<MemoryOperations>, thread: &str) -> Result<()> {
    println!("{} Closing session: {}", "🔒".bold(), thread.cyan());

    // Close the session (triggers SessionClosed event → MemoryEventCoordinator)
    operations.close_session(thread).await?;

    println!("{} Session closed successfully", "✓".green().bold());
    println!("  {}: {}", "Thread ID".cyan(), thread);
    println!();
    println!("{} Waiting for memory extraction, L0/L1 generation, and indexing to complete...", "⏳".yellow().bold());

    // Wait for background tasks to complete (max 60 seconds)
    // This ensures memory extraction, layer generation, and vector indexing finish before CLI exits
    let completed = operations.flush_and_wait(Some(1)).await;

    if completed {
        println!("{} All background tasks completed successfully", "✓".green().bold());
    } else {
        println!("{} Background tasks timed out (some may still be processing)", "⚠".yellow().bold());
    }

    Ok(())
}