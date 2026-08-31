# Spec: Phase 3 axum scaffold + WebSocket sessions (KET-23, KET-24)

Recreate `server/` as `dab-server`. Health endpoints plus a sessioned WS
gateway. No binary protocol, rooms, or React multiplayer UI.

Linear: [KET-23](https://linear.app/sharma01ketan/issue/KET-23/dotsandboxes-server-axum-scaffold-config-health-logging),
[KET-24](https://linear.app/sharma01ketan/issue/KET-24/dotsandboxes-server-websocket-gateway-session-management).

## Goals

- axum + tokio + tracing; `PORT` (default 8080) and `RUST_LOG` from env.
- `GET /health` liveness, `GET /ready` process up; ready JSON includes
  `dab-core` crate name so the native link is real.
- Graceful shutdown (ctrl-c).
- `GET /ws`: upgrade, session id, JSON `{type: hello|ping|pong}` until KET-25.
- Connection registry; remove on disconnect. Heartbeat: server may ping;
  client ping → pong.
- Integration test: connect → hello → ping → close → registry empty.

## Non-goals

- Binary codec → [KET-25](https://linear.app/sharma01ketan/issue/KET-25).
- Rooms / matchmaking / Redis / Postgres.
- Recreating `gpu/` / `ai/` / `proto/` / `infra/`.
- Vercel / frontend deploy of the server.
- React multiplayer client → [KET-34](https://linear.app/sharma01ketan/issue/KET-34).

## Layout

```
server/Cargo.toml
server/src/lib.rs    # router, sessions, health
server/src/main.rs   # bind, tracing, shutdown
```

Workspace member. `cargo test --workspace` covers it.

## Acceptance

- [ ] Server boots; `/health` 200; `/ready` 200 and mentions `dab-core`.
- [ ] Unit test constructs `Game::new` via the core.
- [ ] WS: hello, ping/pong, disconnect clears the registry.
- [ ] `cargo test -p dab-server`; clippy clean.

## Files

| Path | Role |
|------|------|
| `docs/specs/phase3-server-ws.md` | This spec |
| `server/` | axum crate |
| `Cargo.toml` | workspace member |
