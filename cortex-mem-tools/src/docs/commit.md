Commit accumulated conversation content and trigger memory extraction.

**IMPORTANT - Call this tool proactively and periodically, NOT just at conversation end.**

This commits the session and triggers the complete memory processing pipeline:
1. Extracts structured memories (user preferences, entities, decisions)
2. Generates complete L0/L1 layer summaries
3. Indexes all extracted memories into the vector database

**When to call this tool:**
- After completing a significant task or topic discussion
- After the user has shared important preferences or decisions
- When the conversation topic shifts to something new
- After accumulating substantial conversation content (every 10-20 exchanges)
- Before ending a conversation session

**Do NOT wait until the very end of conversation** - the user may forget or the session may end abruptly.

**Guidelines:**
- Call this tool at natural checkpoints in the conversation
- Avoid calling too frequently (not after every message)
- A good rhythm: once per significant topic completion
- This is a long-running operation (30-60s) but runs asynchronously
