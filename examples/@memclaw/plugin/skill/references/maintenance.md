# Maintenance Guide

Periodic maintenance commands for MemClaw data health.

## CLI Tool Location

The `cortex-mem-cli` tool is installed with the platform-specific binary package:

| Platform | CLI Path |
|----------|----------|
| macOS | `{claw-data-dir}/extensions/memclaw/node_modules/@memclaw/bin-darwin-arm64/bin/cortex-mem-cli` |
| Windows | `{claw-data-dir}\extensions\memclaw\node_modules\@memclaw\bin-win-x64\bin\cortex-mem-cli.exe` |

> **Note**: `{claw-data-dir}` is typically `~/.openclaw` for standard OpenClaw. Use your actual Claw data directory for custom versions.

## Maintenance Commands

All commands require `--config` and `--tenant` parameters:

```bash
cortex-mem-cli --config <config-path> --tenant tenant_claw <command>
```

Config file location:
- macOS: `~/Library/Application Support/memclaw/config.toml`
- Windows: `%LOCALAPPDATA%\memclaw\config.toml`

### Vector Maintenance

```bash
# Remove dangling vectors (source files no longer exist)
cortex-mem-cli --config config.toml --tenant tenant_claw vector prune

# Preview without making changes
cortex-mem-cli --config config.toml --tenant tenant_claw vector prune --dry-run

# Rebuild vector index
cortex-mem-cli --config config.toml --tenant tenant_claw vector reindex
```

### Layer Maintenance

```bash
# Generate missing L0/L1 layer files
cortex-mem-cli --config config.toml --tenant tenant_claw layers ensure-all

# Regenerate oversized abstracts
cortex-mem-cli --config config.toml --tenant tenant_claw layers regenerate-oversized
```

## Scheduled Maintenance

**You can set up a scheduled task in OpenClaw to run maintenance automatically:**

### Option 1: OpenClaw Cron Job

Create a Cron Job in OpenClaw that runs every 3 hours:

1. **Schedule**: `0 */3 * * *`
2. **Task**: Execute maintenance commands using the CLI tool
3. **Commands**:
   ```
   cortex-mem-cli --config <config-path> --tenant tenant_claw vector prune
   cortex-mem-cli --config <config-path> --tenant tenant_claw vector reindex
   cortex-mem-cli --config <config-path> --tenant tenant_claw layers ensure-all
   ```

### Option 2: Manual Tool Invocation

Use the `cortex_maintenance` tool for on-demand maintenance:

```json
{ "dryRun": false }
```

## Diagnostic Commands

### Check System Status

```bash
cortex-mem-cli --config config.toml --tenant tenant_claw stats
```

### Check Layer Status

```bash
cortex-mem-cli --config config.toml --tenant tenant_claw layers status
```

### Check Vector Status

```bash
cortex-mem-cli --config config.toml --tenant tenant_claw vector status
```

## Quick Fix Flow

1. **Search not working well?** → Check `layers status` and `vector status`
2. **Missing L0/L1 layers?** → Run `layers ensure-all`
3. **Stale vectors detected?** → Run `vector reindex`
4. **Still having issues?** → Run `vector prune`

## Troubleshooting

| Issue | Solution |
|-------|----------|
| CLI not found | Check the binary path in `{claw-data-dir}/extensions/memclaw/node_modules/@memclaw/bin-{platform}/bin/` |
| Connection refused | Check cortex-mem-service at `localhost:8085` |
| Qdrant issues | Verify Qdrant at `localhost:6333` |
| Layer generation fails | Check LLM API key in config.toml |