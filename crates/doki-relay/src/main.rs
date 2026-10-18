//! `doki-relay serve`: run a relay. See the crate docs and `docs/relay.md`.

use std::process::ExitCode;
use std::time::Duration;

use doki_relay::{Relay, RelayConfig};

const USAGE: &str = "usage: doki-relay --data <folder> [--listen 127.0.0.1:8787] [--retention-days 30] [--max-package-mb 256] [--max-space-mb 2048]";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut data = None;
    let mut listen = "127.0.0.1:8787".to_owned();
    let mut retention_days = 30u64;
    let mut max_package_mb = 256u64;
    let mut max_space_mb = 2048u64;
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{flag} needs a value"));
        let parsed: Result<(), String> = (|| {
            match flag.as_str() {
                "--data" => data = Some(value()?),
                "--listen" => listen = value()?,
                "--retention-days" => {
                    retention_days = value()?.parse().map_err(|e| format!("{e}"))?
                }
                "--max-package-mb" => {
                    max_package_mb = value()?.parse().map_err(|e| format!("{e}"))?
                }
                "--max-space-mb" => max_space_mb = value()?.parse().map_err(|e| format!("{e}"))?,
                "-h" | "--help" => return Err(String::new()),
                other => return Err(format!("unknown option {other}")),
            }
            Ok(())
        })();
        if let Err(msg) = parsed {
            if !msg.is_empty() {
                eprintln!("{msg}");
            }
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    }
    let Some(data) = data else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let mut cfg = RelayConfig::new(data);
    cfg.retention = Duration::from_secs(retention_days * 24 * 3600);
    cfg.max_package_bytes = max_package_mb * 1024 * 1024;
    cfg.max_space_bytes = max_space_mb * 1024 * 1024;
    let relay = match Relay::bind(cfg, &listen) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("could not listen on {listen}: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "doki-relay listening on http://{} (put HTTPS in front for internet use)",
        relay.local_addr()
    );
    relay.run();
    ExitCode::SUCCESS
}
