# Console — Agent Manual

Delta over the repository root `AGENTS.md`. [`DESIGN.md`](DESIGN.md) is the design and
operational contract.

## Purpose

The Hub MCP server: the Hub's activities for agents, as the calling user. The Hub
holds the authority and decides every call.

## Invariants

- Identity: slug `console` in `veoveo_mcp_console::SLUG`, MCP `/console/mcp`, port
  8805. Every other spelling of the slug derives from that constant.
- One permission, `console.use`, which the installation grants to every member. The
  server adds no checks of its own.
- Owns the schema `console`, which the Hub creates, and keeps no tables; the database
  connection holds the process instance only.
- Every tool calls the Hub's own routes for its caller through `mcp::hub::Hub` with
  `ctx.client()?.hub_bearer()`, at the caller's profile; the route builder declares
  `calls_hub`. A Hub refusal is the tool's error. Do not decide what the Hub decides.
- `logs` reads `status` first as its access check, then queries the log store by the
  sandbox ID that status names, without credentials. Keep it one query by the
  `openshell.ai/boundary-pair` label.
- Ready `Routes` bind typed tool requests to their output and permission. Discovery,
  registration, annotations and dispatch derive from those routes.

## Build And Test

- `cargo test -p veoveo-mcp-console`; the tests need no database. `tests/mcp.rs`
  calls the tools through the SDK's mesh against fake Hub routes and a fake log store.
- `cargo test -p veoveo-mcp-console --no-default-features --features contract` checks
  the contract alone.
- `docker build --target check .` runs the gate.
