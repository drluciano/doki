# Relay

The relay lets your devices exchange progress without being online at the same time. It stores encrypted handoff packages until each device fetches them, then expires them. It never decides which save wins, never installs anything, and is not a backup.

## Run one

```sh
cargo build --release -p doki-relay
./target/release/doki-relay --data /var/lib/doki-relay --listen 127.0.0.1:8787
```

Or with Docker:

```sh
docker build -f crates/doki-relay/Dockerfile -t doki-relay .
docker run -d -p 127.0.0.1:8787:8787 -v doki-relay:/data doki-relay
```

| Option | Default | Meaning |
|---|---|---|
| `--data` | (required) | Folder for stored packages |
| `--listen` | `127.0.0.1:8787` | Address and port |
| `--retention-days` | `30` | Packages older than this are removed |
| `--max-package-mb` | `256` | Largest accepted package |
| `--max-space-mb` | `2048` | Storage per set of paired devices |

The relay speaks plain HTTP. For anything beyond your own network, put it behind a reverse proxy with HTTPS (Caddy, nginx). Package contents are encrypted on your devices either way, but the access token travels in each request.

## Pair devices

```sh
doki relay create https://relay.example.com    # first device; prints a pairing code
doki relay join DOKI1:https://relay.example.com#…   # each other device
doki sync                                       # on any device, any time
```

`doki sync` sends your current save of each game if it changed since the last sync, then receives other devices' progress into your vault. It never changes save files: review incoming progress with `doki incoming` and `doki use`.

## Security model (interim)

- The pairing code holds a random secret. HKDF-SHA256 derives an access token (the relay stores only its SHA-256) and a separate content key that never leaves your devices.
- Packages are sealed with XChaCha20-Poly1305 before upload, with the space id as associated data. The relay sees only sizes and upload times. Altered or foreign packages fail to open and are skipped.
- All paired devices share one secret. To shut out a lost device, create a new space with `doki relay forget` and `doki relay create` on a remaining device, then re-join the others. Per-device keys, revocation and key recovery are required before the relay is recommended for use over the internet.

## API

Documented in `crates/doki-relay/src/lib.rs`.
