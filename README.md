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

---

## Usage

- Download latest release and run it
- On first run, you will be asked for URL to API (OpenAI or compatible).
- The model names you enter should be valid and your API should be able to serve them.
- Put your documents into the `docs/` folder. They should be `.txt` files, and they are processed the moment they are copied to the folder. Therefore you must make sure your text file is complete before copying them over to docs folder.
- Documents are then automatically embedded into the vector database for later retrieval for prompts behind the scenes.
- Last step, you just change the API URL in your current user facing applications from `http://localhost:11434` to `http://localhost:11435`. A simple port change in this case would do it. (ollama runs on port 11434 on localhost by default, RAGnar runs on 11435)

A simple curl to test this program:

```bash
# No RAG, your old API
curl http://localhost:11434/v1/chat/completions   -H "Content-Type: application/json"   -d '{"model": "tinyllama","stream": false,"messages": [{"role": "system","content": "You are helpful"},{"role": "user","content": "what does acetaminophen do in CKD patients?"}]}'

# With RAGnar
curl http://localhost:11435/v1/chat/completions   -H "Content-Type: application/json"   -d '{"model": "tinyllama","stream": false,"messages": [{"role": "system","content": "You are helpful"},{"role": "user","content": "what does acetaminophen do in CKD patients?"}]}'
```

You can add a specific document in the form of a text file to docs folder, then ask a revealing question to see the difference in the knowledge the two different requests demonstrate.
