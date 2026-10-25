#!/usr/bin/env python3
"""
Notebook Minimal Query Engine
CLI-based RAG and persistent memory handler.
"""

import argparse
import json
import sys
import os
from datetime import datetime

MEMORY_DB = os.path.expanduser("~/.local/share/notebook-minimal/memory.db")
PROVIDER_API = "https://openrouter.ai/api/v1/chat/completions"


def get_embedding_mock(text: str) -> list[float]:
    """MOCK: Replace with actual embedding model call."""
    return [0.1 * (hash(text + str(i)) % 100) / 100 for i in range(384)]


def search_memory_mock(query: str, top_k: int = 5) -> list[dict]:
    """MOCK: Replace with actual SQLite/DuckDB semantic search."""
    print(f"[MOCK DB] Searching: {query[:50]}...", file=sys.stderr)
    return [
        {"id": "1", "text": f"Document about {query}", "score": 0.92, "source": "notes"},
        {"id": "2", "text": f"Related: {query} context", "score": 0.87, "source": "memories"},
    ]


def call_llm_mock(context_chunks: list[str], query: str) -> str:
    """MOCK: Replace with actual OpenRouter/MiniMax API call."""
    context = "\n---\n".join(context_chunks)
    payload = {
        "model": "minimax/minimax-m2.5",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant. Answer based on context."},
            {"role": "user", "content": f"Context:\n{context}\n\nQuestion: {query}"}
        ],
        "max_tokens": 500
    }
    print(f"[MOCK API] Would send to {PROVIDER_API}", file=sys.stderr)
    print(f"[MOCK API] Payload size: {len(json.dumps(payload))} bytes", file=sys.stderr)
    return f"MOCK RESPONSE: Based on the retrieved documents, the answer to '{query}' involves analyzing the stored context and generating a coherent response. (Connect to OpenRouter for live results)"


def action_add(payload: dict) -> None:
    """Add knowledge to persistent memory."""
    text = payload.get("text", "")
    tags = payload.get("tags", [])
    
    if not text:
        print("ERROR: Missing 'text' in payload", file=sys.stderr)
        sys.exit(1)
    
    embedding = get_embedding_mock(text)
    
    entry = {
        "id": f"{datetime.now().timestamp()}",
        "text": text,
        "tags": tags,
        "embedding": embedding,
        "created_at": datetime.now().isoformat()
    }
    
    os.makedirs(os.path.dirname(MEMORY_DB), exist_ok=True)
    
    print(f"[ADD] Stored entry: {entry['id']}")
    print(f"[ADD] Text length: {len(text)} chars")
    print(f"[ADD] Tags: {tags}")
    print(json.dumps({"status": "added", "id": entry["id"]}))


def action_query(payload: dict) -> None:
    """Query knowledge base with RAG."""
    query = payload.get("query", "")
    
    if not query:
        print("ERROR: Missing 'query' in payload", file=sys.stderr)
        sys.exit(1)
    
    results = search_memory_mock(query)
    context = [r["text"] for r in results]
    
    answer = call_llm_mock(context, query)
    
    output = {
        "query": query,
        "results": results,
        "answer": answer
    }
    
    print(json.dumps(output))


def main():
    parser = argparse.ArgumentParser(description="Notebook Minimal Query Engine")
    parser.add_argument("--action", required=True, choices=["add", "query"], help="Action to perform")
    parser.add_argument("--payload", required=True, help="JSON payload")
    args = parser.parse_args()
    
    try:
        payload = json.loads(args.payload)
    except json.JSONDecodeError as e:
        print(f"ERROR: Invalid JSON payload: {e}", file=sys.stderr)
        sys.exit(1)
    
    if args.action == "add":
        action_add(payload)
    elif args.action == "query":
        action_query(payload)


if __name__ == "__main__":
    main()