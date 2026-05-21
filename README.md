# RAGnar

> A Rust-based hybrid RAG proxy for any OpenAI-compatible LLM backend.  
> Intercepts prompts, retrieves grounded context from your private knowledge base, and forwards enriched requests — locally and privately.


## Why RAGnar?

Large language models are impressive. But they cannot answer questions about your internal documents, they cite nothing, and private data cannot leave your infrastructure.

RAGnar sits between your application and any LLM backend. Every prompt is automatically enriched with relevant, source-attributed context retrieved from your own corpus before the model ever sees it.

Built for environments where the answer needs to be **verifiable, private, and grounded** — medical, legal, and enterprise use cases where *"the model said so"* is not sufficient.


## Features

- **Drop-in proxy** — speaks the OpenAI chat completions API on both sides; no client changes required
- **Vector RAG** — vector similarity search for retrieval, optimized with context of surrounding chunks
- **Fully local** — runs entirely on your infrastructure; patient data, clinical notes, and proprietary documents never leave your network
- **Source attribution** — every response includes the retrieved source chunks so answers can be verified and audited
- **Interactive setup** — `ragnar init` provides a terminal UI wizard that generates your config file


## Architecture

```
Client (any OpenAI-compatible app)
        │
        │  POST /v1/chat/completions
        ▼
┌──────────────────────────────────────┐
│                RAGnar                │
│                                      │
│  1. Parse incoming prompt            │
│  2. Embed query                      │
│  3. Vector search                    │
│  4. Rewrite query                    │
│  6. Forward to backend               │
└──────────────────────────────────────┘
        │
        │  Enriched prompt
        ▼
  LLM Backend (Ollama / OpenAI / ...)
        │
        ▼
  Response (with source attribution metadata)
        │
        ▼
      Client
```
**Note**: Each request is independently constructed. The system manages conversational context by maintaining a sliding window and resetting (or summarizing) history when token limits are reached, while RAG retrieval is performed on every turn.

---

## Roadmap

- [ ] OpenAI-compatible proxy
- [ ] Document watcher
- [ ] Vector RAG (LanceDB)
- [ ] ratatui init wizard

---
