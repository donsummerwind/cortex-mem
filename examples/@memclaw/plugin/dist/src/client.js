"use strict";
/**
 * Cortex Mem Client
 *
 * HTTP client for cortex-mem-service REST API.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.CortexMemClient = void 0;
class CortexMemClient {
    baseUrl;
    constructor(baseUrl = 'http://localhost:8085') {
        this.baseUrl = baseUrl;
    }
    // ==================== Search ====================
    /**
     * Layered semantic search with L0/L1/L2 tiered retrieval
     *
     * @param options.scope - URI prefix to limit search scope:
     *   - "cortex://session/abc" - search within a specific session
     *   - "cortex://user/default" - search user memories
     *   - "cortex://agent/claw/cases" - search agent cases
     *   - Omit to search across all dimensions
     */
    async search(options) {
        // Convert scope to root_uri for backend API
        // Backend expects root_uri parameter for URI prefix filtering
        const scope = options.scope;
        const response = await this.fetchJson('/api/v2/search', {
            method: 'POST',
            body: JSON.stringify({
                query: options.query,
                thread: scope, // Backend still accepts thread for backward compatibility
                limit: options.limit ?? 10,
                min_score: options.min_score ?? 0.6,
                return_layers: options.return_layers ?? ['L0']
            })
        });
        if (!response.success || !response.data) {
            throw new Error(response.error ?? 'Search failed');
        }
        return response.data;
    }
    /**
     * Recall memories with more context (L0 + L2)
     */
    async recall(query, scope, limit = 10) {
        return this.search({
            query,
            scope,
            limit,
            return_layers: ['L0', 'L2']
        });
    }
    // ==================== Filesystem ====================
    /**
     * List directory contents
     */
    async ls(options = {}) {
        const params = new URLSearchParams();
        params.set('uri', options.uri ?? 'cortex://session');
        if (options.recursive)
            params.set('recursive', 'true');
        if (options.include_abstracts)
            params.set('include_abstracts', 'true');
        const response = await this.fetchJson(`/api/v2/filesystem/list?${params.toString()}`);
        if (!response.success || !response.data) {
            throw new Error(response.error ?? 'List directory failed');
        }
        return response.data;
    }
    /**
     * Smart exploration combining search and browsing
     */
    async explore(options) {
        const response = await this.fetchJson('/api/v2/filesystem/explore', {
            method: 'POST',
            body: JSON.stringify({
                query: options.query,
                start_uri: options.start_uri ?? 'cortex://session',
                return_layers: options.return_layers ?? ['L0']
            })
        });
        if (!response.success || !response.data) {
            throw new Error(response.error ?? 'Explore failed');
        }
        return response.data;
    }
    // ==================== Tiered Access ====================
    /**
     * Get L0 abstract (~100 tokens) for quick relevance check
     */
    async getAbstract(uri) {
        const params = new URLSearchParams();
        params.set('uri', uri);
        const response = await this.fetchJson(`/api/v2/filesystem/abstract?${params.toString()}`);
        if (!response.success || !response.data) {
            throw new Error(response.error ?? 'Get abstract failed');
        }
        return response.data;
    }
    /**
     * Get L1 overview (~2000 tokens) for core information
     */
    async getOverview(uri) {
        const params = new URLSearchParams();
        params.set('uri', uri);
        const response = await this.fetchJson(`/api/v2/filesystem/overview?${params.toString()}`);
        if (!response.success || !response.data) {
            throw new Error(response.error ?? 'Get overview failed');
        }
        return response.data;
    }
    /**
     * Get L2 full content
     */
    async getContent(uri) {
        const params = new URLSearchParams();
        params.set('uri', uri);
        const response = await this.fetchJson(`/api/v2/filesystem/content?${params.toString()}`);
        if (!response.success || !response.data) {
            throw new Error(response.error ?? 'Get content failed');
        }
        return response.data;
    }
    // ==================== Session Management ====================
    /**
     * List all sessions
     */
    async listSessions() {
        const response = await this.fetchJson('/api/v2/sessions');
        if (!response.success || !response.data) {
            throw new Error(response.error ?? 'List sessions failed');
        }
        return response.data;
    }
    /**
     * Add a message to a session
     */
    async addMessage(threadId, message) {
        const response = await this.fetchJson(`/api/v2/sessions/${threadId}/messages`, {
            method: 'POST',
            body: JSON.stringify({
                role: message.role ?? 'user',
                content: message.content,
                metadata: message.metadata
            })
        });
        if (!response.success || !response.data) {
            throw new Error(response.error ?? 'Add message failed');
        }
        return response.data;
    }
    /**
     * Commit a session and trigger memory extraction
     */
    async commitSession(threadId) {
        const response = await this.fetchJson(`/api/v2/sessions/${threadId}/close`, {
            method: 'POST',
            body: JSON.stringify({})
        });
        if (!response.success || !response.data) {
            throw new Error(response.error ?? 'Commit session failed');
        }
        return response.data;
    }
    // ==================== Tenant ====================
    /**
     * Switch tenant context
     */
    async switchTenant(tenantId) {
        const response = await this.fetchJson('/api/v2/tenants/switch', {
            method: 'POST',
            body: JSON.stringify({ tenant_id: tenantId })
        });
        if (!response.success) {
            throw new Error(response.error ?? 'Switch tenant failed');
        }
    }
    // ==================== Internal ====================
    async fetchJson(path, options = {}) {
        const url = `${this.baseUrl}${path}`;
        const headers = {
            'Content-Type': 'application/json',
            ...(options.headers || {})
        };
        const response = await fetch(url, {
            ...options,
            headers
        });
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        return response.json();
    }
}
exports.CortexMemClient = CortexMemClient;
//# sourceMappingURL=client.js.map