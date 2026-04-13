---
name: dev
description: Start local development environment
disable-model-invocation: true
argument-hint: [backend|web|desktop|all]
allowed-tools: Bash
---

# Start Development Environment

Start the specified service locally ($ARGUMENTS or "all").

## Backend
```bash
just backend-dev
```
Runs on http://localhost:8080. Loads API keys from `~/.wolfgang/.env`.
Set `DISABLE_AUTH=true` in that file to skip Firebase auth locally.

## Web
```bash
just web-dev
```
Runs on http://localhost:5173. Proxies `/api/chat` to backend on :8080.
The backend must be running for chat to work.

## Desktop
```bash
just desktop-dev
```
Standalone Tauri app. Calls Anthropic/OpenAI directly. No backend needed.

## All
Start backend in background, then web in foreground.
Desktop is separate (Tauri has its own dev server).

When starting "all", remind the user to have `~/.wolfgang/.env` set up with:
```
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...
DISABLE_AUTH=true
```
