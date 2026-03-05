#!/bin/bash
# Cortex-Mem CLI 测试数据生成脚本

set -e

# 配置
DATA_DIR="${CORTEX_DATA_DIR:-./.cortex}"
TENANT="${CORTEX_TENANT:-default}"
SESSION_ID="test-session-$(date +%Y%m%d%H%M%S)"

echo "🚀 Cortex-Mem CLI 测试数据生成器"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📂 数据目录: $DATA_DIR"
echo "🏢 租户ID: $TENANT"
echo "💬 会话ID: $SESSION_ID"
echo ""

# 🔧 修复：使用租户模式路径
# CLI 使用 with_tenant() 创建文件系统，路径为: {root}/tenants/{tenant_id}/
TENANT_BASE="$DATA_DIR/tenants/$TENANT"

# 创建目录结构
SESSION_DIR="$TENANT_BASE/session/$SESSION_ID"
TIMELINE_DIR="$SESSION_DIR/timeline/$(date +%Y-%m)/$(date +%d)"

echo "📁 创建目录结构..."
echo "   租户路径: $TENANT_BASE"
mkdir -p "$TIMELINE_DIR"

# 创建会话元数据
cat > "$SESSION_DIR/.session.json" << EOF
{
  "session_id": "$SESSION_ID",
  "user_id": "test-user",
  "agent_id": "test-agent",
  "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "updated_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "metadata": {}
}
EOF
echo "✅ 创建会话元数据: $SESSION_DIR/.session.json"

# 创建测试消息
for i in {1..5}; do
  MSG_ID=$(uuidgen | tr '[:upper:]' '[:lower:]' | cut -d'-' -f1)
  TIMESTAMP=$(date -u +"%H_%M_%S")_$MSG_ID
  MSG_FILE="$TIMELINE_DIR/${TIMESTAMP}.md"

  ROLE=$( [ $((i % 2)) -eq 0 ] && echo "assistant" || echo "user" )

  cat > "$MSG_FILE" << EOF
# $ROLE Message

**ID**: \`$MSG_ID\`
**Timestamp**: $(date -u +"%Y-%m-%d %H:%M:%S UTC")

## 内容

这是第 $i 条测试消息。这条消息包含足够的文本来生成有意义的 L0 抽象和 L1 概览。

### 主题
- Cortex Memory 3.0 的层级检索功能
- 三层递进架构（L0/L1/L2）
- 分布式记忆管理

### 详细内容
Cortex Memory 采用了三层递进架构：
- **L0 (Abstract)**: 简洁摘要，~100 tokens，用于快速过滤
- **L1 (Overview)**: 结构化概览，~500-2000 tokens，用于决策
- **L2 (Detail)**: 完整内容，原始数据

这种设计能够在大规模记忆库中高效检索相关信息。
EOF

  echo "✅ 创建消息 $i: $(basename $MSG_FILE)"
  sleep 0.1
done

# 创建用户维度测试数据
USER_DIR="$TENANT_BASE/user/test-user/preferences"
mkdir -p "$USER_DIR"

cat > "$USER_DIR/pref_0.md" << 'EOF'
# 编程语言偏好

用户偏好使用 Rust 进行系统编程，喜欢类型安全和性能优化。

**Added**: 2026-02-25 16:00:00 UTC
**Confidence**: 0.95
EOF
echo "✅ 创建用户偏好: $USER_DIR/pref_0.md"

# 创建 Agent 维度测试数据
AGENT_DIR="$TENANT_BASE/agent/test-agent/cases"
mkdir -p "$AGENT_DIR"

cat > "$AGENT_DIR/case_0.md" << 'EOF'
# 解决 Rust 编译错误

## Problem

用户遇到了 `use of unresolved module` 错误。

## Solution

在 `Cargo.toml` 中添加缺失的依赖 `futures = { workspace = true }`。

## Lessons Learned

- 始终检查 workspace 依赖是否正确引用
- 使用 `cargo check` 快速验证编译问题

**Added**: 2026-02-25 16:00:00 UTC
**Confidence**: 0.90
EOF
echo "✅ 创建 Agent 案例: $AGENT_DIR/case_0.md"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✨ 测试数据生成完成！"
echo ""
echo "📊 统计信息:"
echo "   • 会话消息: 5 条"
echo "   • 用户偏好: 1 条"
echo "   • Agent案例: 1 条"
echo ""
echo "🧪 下一步测试命令:"
echo "   1. 查看状态:   cargo run -p cortex-mem-cli -- layers status"
echo "   2. 生成层级:   cargo run -p cortex-mem-cli -- layers ensure-all"
echo "   3. 查看会话:   cargo run -p cortex-mem-cli -- list -u cortex://session/$SESSION_ID"
echo ""
echo "📂 数据目录: $TENANT_BASE"
echo "💡 提示: CLI 使用租户模式，数据存储在 {data_dir}/tenants/{tenant_id}/"
