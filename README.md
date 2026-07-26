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
kucoin = "api.kucoin.com"
```

Changes are picked up automatically — no restart needed. The file is
gitignored since it usually lists real, possibly sensitive hosts.

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
