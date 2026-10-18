# Self-hosted relay

Direct sync needs both devices online at once. If yours rarely are, a relay can hold progress until each device fetches it. You run it yourself. It is optional, and most people will not need it.

The relay stores only encrypted packages. It never sees your saves or game names, never decides which save wins, never installs anything, and is not a backup. Packages expire after a retention period (30 days by default).

## Run it

With Docker:

```sh
docker build -f crates/doki-relay/Dockerfile -t doki-relay .
docker run -d -p 127.0.0.1:8787:8787 -v doki-relay:/data doki-relay
```

Or from source:

```sh
cargo build --release -p doki-relay
./target/release/doki-relay --data /var/lib/doki-relay --listen 127.0.0.1:8787
```

The relay speaks plain HTTP. To reach it from outside your home network, put it behind a reverse proxy with HTTPS (Caddy, nginx).

Options (`--retention-days`, `--max-package-mb`, `--max-space-mb`) are listed in [docs/relay.md](https://github.com/drluciano/doki/blob/main/docs/relay.md).

## Connect your devices

In the app, open **Devices → Advanced: your own relay server**, choose **Create a relay code** on the first device, and **Join a relay** with that code on the others.

On the command line:

```sh
doki relay create https://relay.example.com   # first device; prints a code
doki relay join DOKI1:…                        # each other device
doki sync                                      # any time
```

Received progress waits in Incoming, as with direct sync.

## Current limits

All devices on one relay share one secret. To shut out a lost device, choose **Forget this relay** (`doki relay forget`) on a remaining device, create a new code and re-join the others. Per-device relay keys are planned before the relay is recommended for internet-facing use.
