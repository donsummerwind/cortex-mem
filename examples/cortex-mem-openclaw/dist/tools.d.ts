/**
 * Tool definitions for Cortex Memory plugin
 */
import type { ContextLayer } from './client.js';
export declare const toolSchemas: {
    readonly cortex_search: {
        readonly name: "cortex_search";
        readonly description: "Layered semantic search across memory using L0/L1/L2 tiered retrieval.\nReturns relevant memories ranked by relevance score.\n\nUse this tool when you need to:\n- Find past conversations or decisions\n- Search for specific information across all sessions\n- Discover related memories by semantic similarity";
        readonly inputSchema: {
            readonly type: "object";
            readonly properties: {
                readonly query: {
                    readonly type: "string";
                    readonly description: "The search query - can be natural language or keywords";
                };
                readonly scope: {
                    readonly type: "string";
                    readonly description: "Optional session/thread ID to limit search scope";
                };
                readonly limit: {
                    readonly type: "integer";
                    readonly description: "Maximum number of results to return (default: 10)";
                    readonly default: 10;
                };
                readonly min_score: {
                    readonly type: "number";
                    readonly description: "Minimum relevance score threshold (0-1, default: 0.6)";
                    readonly default: 0.6;
                };
            };
            readonly required: readonly ["query"];
        };
    };
    readonly cortex_recall: {
        readonly name: "cortex_recall";
        readonly description: "Recall memories with layered detail levels (L0/L1/L2).\n\nL0 (Abstract): ~100 tokens - Quick summary for relevance判断\nL1 (Overview): ~2000 tokens - Key points and context\nL2 (Full): Complete content - Use only when you need full details\n\nUse this when you need more context than what cortex_search provides.";
        readonly inputSchema: {
            readonly type: "object";
            readonly properties: {
                readonly query: {
                    readonly type: "string";
                    readonly description: "The search query";
                };
                readonly layers: {
                    readonly type: "array";
                    readonly items: {
                        readonly type: "string";
                        readonly enum: readonly ["L0", "L1", "L2"];
                    };
                    readonly description: "Which detail layers to return (default: [\"L0\"])";
                    readonly default: readonly ["L0"];
                };
                readonly scope: {
                    readonly type: "string";
                    readonly description: "Optional session/thread ID to limit search scope";
                };
                readonly limit: {
                    readonly type: "integer";
                    readonly description: "Maximum number of results (default: 5)";
                    readonly default: 5;
                };
            };
            readonly required: readonly ["query"];
        };
    };
    readonly cortex_add_memory: {
        readonly name: "cortex_add_memory";
        readonly description: "Add a message to memory for a specific session.\nThis stores the message and automatically triggers:\n- Vector embedding for semantic search\n- L0/L1 layer generation (async)\n\nUse this to persist important information that should be searchable later.";
        readonly inputSchema: {
            readonly type: "object";
            readonly properties: {
                readonly content: {
                    readonly type: "string";
                    readonly description: "The content to store in memory";
                };
                readonly role: {
                    readonly type: "string";
                    readonly enum: readonly ["user", "assistant", "system"];
                    readonly description: "Role of the message sender (default: user)";
                    readonly default: "user";
                };
                readonly session_id: {
                    readonly type: "string";
                    readonly description: "Session/thread ID (uses default if not specified)";
                };
            };
            readonly required: readonly ["content"];
        };
    };
    readonly cortex_list_sessions: {
        readonly name: "cortex_list_sessions";
        readonly description: "List all memory sessions with their status.\nShows session IDs, message counts, and creation/update times.";
        readonly inputSchema: {
            readonly type: "object";
            readonly properties: {};
        };
    };
};
export interface CortexSearchInput {
    query: string;
    scope?: string;
    limit?: number;
    min_score?: number;
}
export interface CortexRecallInput {
    query: string;
    layers?: ContextLayer[];
    scope?: string;
    limit?: number;
}
export interface CortexAddMemoryInput {
    content: string;
    role?: 'user' | 'assistant' | 'system';
    session_id?: string;
}
export interface CortexListSessionsInput {
}
export interface CortexSearchOutput {
    results: Array<{
        uri: string;
        score: number;
        snippet: string;
    }>;
    total: number;
}
export interface CortexRecallOutput {
    results: Array<{
        uri: string;
        score: number;
        abstract?: string;
        overview?: string;
        content?: string;
    }>;
    total: number;
}
export interface CortexAddMemoryOutput {
    success: boolean;
    message_uri: string;
}
export interface CortexListSessionsOutput {
    sessions: Array<{
        thread_id: string;
        status: string;
        message_count: number;
        created_at: string;
    }>;
}
//# sourceMappingURL=tools.d.ts.map