List directory contents to browse the memory space like a virtual filesystem.

This allows you to explore the hierarchical structure of memories:
- cortex://session - List all sessions
- cortex://session/{session_id} - Browse a specific session's contents
- cortex://session/{session_id}/timeline - View timeline messages
- cortex://session/{session_id}/memories - View extracted memories
- cortex://user - View user-level memories (preferences, entities, goals)
- cortex://agent - View agent-level memories

**Parameters:**
- recursive: List all subdirectories recursively
- include_abstracts: Show L0 abstracts for each file (for quick preview)

Use this when:
- Semantic search doesn't find what you need
- You want to understand the overall memory layout
- You need to manually navigate to find specific information
