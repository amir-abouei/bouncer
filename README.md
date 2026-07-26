# bouncer

Tiny allowlist-based reverse proxy. Requests to `/<alias>/<path>` are
forwarded to the upstream host mapped to `<alias>` in
`config/allowlist.toml`. Unlisted aliases get `403`.

## Quick start

```bash
cp .env.example .env
cp config/allowlist.example.toml config/allowlist.toml
docker compose up --build -d
```

```bash
curl http://localhost:8080/healthz
curl "http://localhost:8080/kucoin/api/v1/timestamp"
```

## Configuring targets

Edit `config/allowlist.toml`:

```toml
[targets]
kucoin = { host = "api.kucoin.com" }
```

Changes are picked up automatically — no restart needed. The file is
gitignored since it usually lists real, possibly sensitive hosts.

## Requiring auth on a route

Add `auth = true` to any target to lock it behind a JWT:

```toml
[targets]
kucoin = { host = "api.kucoin.com" }                          # open, auth defaults to false
webhook = { host = "internal.example.com", auth = true }       # requires a JWT
```

If any target has `auth = true`, you must set `JWT_SECRET` (an HS256 shared
secret) in the environment — bouncer refuses to start otherwise, so a route
can never end up "protected" by accident with nothing actually checking it.

Requests to an `auth = true` alias must carry `X-Bouncer-Token: <token>`.
This is a separate header from `Authorization` on purpose — `Authorization`
is reserved for whatever credentials the upstream target itself expects
(an API key, its own bearer token, etc.) and bouncer forwards it through
untouched; `X-Bouncer-Token` is stripped before the request leaves bouncer,
so the upstream never sees it. bouncer verifies only the HS256 signature —
no `exp`/`nbf`/`aud`/`iss`/`sub` claim is checked, so a minted token stays
valid indefinitely and there's nothing to rotate on a schedule. The
trade-off: there's no per-token revocation — the only way to invalidate a
token is to rotate `JWT_SECRET` itself, which invalidates every token at
once (and requires a restart). Missing or badly-signed tokens get a `401`
with a generic body — the specific reason is only logged server-side,
never returned to the caller.

bouncer only **verifies** tokens — it doesn't issue them. Sign your own
tokens with whatever process/library you like using the same secret as
`JWT_SECRET`. For testing, a minting helper is included:

```bash
JWT_SECRET=your-secret cargo run --example mint_token
```

prints a signed HS256 token to stdout, ready to use as:

```bash
curl -H "X-Bouncer-Token: $(JWT_SECRET=your-secret cargo run -q --example mint_token)" \
  http://localhost:8080/webhook/...
```

`auth = true`/`false` on existing or new aliases hot-reloads the same way
allowlist changes always have. `JWT_SECRET` itself does not hot-reload —
rotating it requires a restart, same as `LISTEN_ADDR`/`ALLOWLIST_PATH`.
`/healthz` is never auth-gated.

## Local dev (without Docker)

```bash
cargo build --release
cp -n config/allowlist.example.toml config/allowlist.toml
ALLOWLIST_PATH=config/allowlist.toml ./target/release/bouncer
```

## Environment variables

| Variable         | Default                 | Meaning                        |
|------------------|--------------------------|---------------------------------|
| `LISTEN_ADDR`    | `0.0.0.0:8080`           | Bind address                    |
| `ALLOWLIST_PATH` | `config/allowlist.toml`  | Path to the allowlist config    |
| `RUST_LOG`       | `info`                   | Log level                       |
| `JWT_SECRET`     | unset                    | HS256 shared secret; required only if a target has `auth = true` |
| `HOST_PORT`      | `8080`                   | (Compose only) host port        |
| `IMAGE`          | `bouncer:local`          | (Compose only) image to build/pull |

## Notes

- Hop-by-hop headers are stripped; bodies are streamed through and capped
  at 100 MB (`MAX_BODY_BYTES` in `src/main.rs`).
- TLS is pure Rust (`hyper-rustls` + `ring`) — no OpenSSL required.
- CI publishes images to GHCR on push to `main`
  (`.github/workflows/docker-publish.yml`). Set
  `IMAGE=ghcr.io/<you>/bouncer:latest` in `.env` and run
  `docker compose pull && docker compose up -d --no-build` to skip
  building locally.
