Layered semantic search across memory using L0/L1/L2 tiered retrieval.

**Key Features:**
- Tiered retrieval: L0 (abstract) -> L1 (overview) -> L2 (full content)
- Token-efficient: Control exactly which layers to return

**Parameters:**
- return_layers: ["L0"] (default, ~100 tokens), ["L0","L1"] (~2100 tokens), ["L0","L1","L2"] (full)

**When to use:**
- Finding past conversations or decisions
- Searching across all sessions
- Discovering related memories by semantic similarity
