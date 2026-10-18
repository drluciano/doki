//! Doki relay: store-and-forward for encrypted handoff packages.
//!
//! The relay lets devices exchange progress without being online at the same
//! time. It is deliberately simple and untrusted:
//!
//! - It stores opaque blobs. Devices encrypt packages before upload, so the
//!   relay never sees save bytes, file names or game names.
//! - It never decides which save wins and never asks a device to delete
//!   anything. Expiring an old blob is not a deletion request.
//! - It is not a backup. Blobs expire after the retention period.
//!
//! # API (version 1)
//!
//! All `/v1/spaces/{space}` routes require `Authorization: Bearer {token}`.
//! A space is one person's set of paired devices. `{space}` is 16–64
//! lowercase hex characters; `{token}` is 64 lowercase hex characters
//! derived by devices from their pairing secret. The relay stores only a
//! SHA-256 of the token.
//!
//! | Method and path | Purpose | Responses |
//! |---|---|---|
//! | `GET /v1/health` | Liveness | 200 `{"ok":true}` |
//! | `PUT /v1/spaces/{space}` | Register the space on first use, or check the token | 200, 401 |
//! | `POST /v1/spaces/{space}/packages` | Store a blob; header `X-Doki-Device: {hex id}` | 200 `{"seq":n}`, 401, 404, 413, 507 |
//! | `GET /v1/spaces/{space}/packages?after={seq}` | List blobs with a higher sequence number | 200 `[PackageInfo]` |
//! | `GET /v1/spaces/{space}/packages/{seq}` | Download one blob | 200 bytes, 404 |
//!
//! Errors are JSON `{"error": "<code>"}` with codes `unauthorized`,
//! `not_found`, `bad_request`, `too_large`, `quota_exceeded`, `internal`.
//!
//! # Storage
//!
//! `{data}/spaces/{space}/space.json` holds the token hash and next sequence
//! number; `{seq}.bin` and `{seq}.json` hold each blob and its metadata.
//! Writes go to a temporary file and are renamed into place.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tiny_http::{Header, Method, Request, Response, Server};

#[derive(Debug, Clone)]
pub struct RelayConfig {
    pub data_dir: PathBuf,
    /// Largest accepted blob.
    pub max_package_bytes: u64,
    /// Total stored bytes per space.
    pub max_space_bytes: u64,
    /// Blobs older than this are removed.
    pub retention: Duration,
}

impl RelayConfig {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        RelayConfig {
            data_dir: data_dir.into(),
            max_package_bytes: 256 * 1024 * 1024,
            max_space_bytes: 2 * 1024 * 1024 * 1024,
            retention: Duration::from_secs(30 * 24 * 3600),
        }
    }
}

/// Metadata for one stored blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageInfo {
    pub seq: u64,
    /// Device id the uploader declared. Authenticated only as "holds the
    /// space token"; devices must not trust it for anything but skipping
    /// their own uploads.
    pub from_device: String,
    pub size: u64,
    pub created_unix: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SpaceMeta {
    token_sha256: String,
    next_seq: u64,
    created_unix: u64,
}

pub struct Relay {
    server: Server,
    cfg: RelayConfig,
}

fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn is_hex(s: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&s.len())
        && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Compare without stopping at the first difference.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

type Reply = Response<io::Cursor<Vec<u8>>>;

fn json<T: Serialize>(status: u16, value: &T) -> Reply {
    Response::from_data(serde_json::to_vec(value).unwrap())
        .with_status_code(status)
        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap())
}

fn error(status: u16, code: &str) -> Reply {
    json(status, &serde_json::json!({ "error": code }))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut f = File::create(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    fs::rename(tmp, path)
}

impl Relay {
    /// Bind to `addr` (use port 0 for an ephemeral port).
    pub fn bind(cfg: RelayConfig, addr: &str) -> io::Result<Self> {
        fs::create_dir_all(cfg.data_dir.join("spaces"))?;
        let server = Server::http(addr).map_err(|e| io::Error::other(e.to_string()))?;
        Ok(Relay { server, cfg })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.server.server_addr().to_ip().expect("bound to an IP address")
    }

    /// Serve requests one at a time until the process exits.
    pub fn run(&self) {
        for request in self.server.incoming_requests() {
            self.handle(request);
        }
    }

    fn handle(&self, mut request: Request) {
        let reply = self.route(&mut request).unwrap_or_else(|e| {
            eprintln!("doki-relay: internal error: {e}");
            error(500, "internal")
        });
        let _ = request.respond(reply);
    }

    fn route(&self, request: &mut Request) -> io::Result<Reply> {
        let url = request.url().to_owned();
        let (path, query) = url.split_once('?').unwrap_or((&url, ""));
        let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
        match (request.method(), parts.as_slice()) {
            (Method::Get, ["v1", "health"]) => Ok(json(200, &serde_json::json!({ "ok": true }))),
            (Method::Put, ["v1", "spaces", space]) => self.register(request, space),
            (Method::Post, ["v1", "spaces", space, "packages"]) => self.upload(request, space),
            (Method::Get, ["v1", "spaces", space, "packages"]) => {
                let after = query
                    .split('&')
                    .find_map(|kv| kv.strip_prefix("after="))
                    .map(|v| v.parse::<u64>())
                    .transpose();
                match after {
                    Ok(after) => self.list(request, space, after.unwrap_or(0)),
                    Err(_) => Ok(error(400, "bad_request")),
                }
            }
            (Method::Get, ["v1", "spaces", space, "packages", seq]) => match seq.parse::<u64>() {
                Ok(seq) => self.download(request, space, seq),
                Err(_) => Ok(error(400, "bad_request")),
            },
            _ => Ok(error(404, "not_found")),
        }
    }

    fn space_dir(&self, space: &str) -> PathBuf {
        self.cfg.data_dir.join("spaces").join(space)
    }

    fn bearer(request: &Request) -> Option<String> {
        request
            .headers()
            .iter()
            .find(|h| h.field.equiv("Authorization"))
            .and_then(|h| h.value.as_str().strip_prefix("Bearer ").map(str::to_owned))
            .filter(|t| is_hex(t, 64, 64))
    }

    fn token_hash(token: &str) -> String {
        hex::encode(Sha256::digest(token.as_bytes()))
    }

    fn load_meta(&self, space: &str) -> io::Result<Option<SpaceMeta>> {
        let path = self.space_dir(space).join("space.json");
        match fs::read(&path) {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes).map_err(io::Error::other)?)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn save_meta(&self, space: &str, meta: &SpaceMeta) -> io::Result<()> {
        write_atomic(&self.space_dir(space).join("space.json"), &serde_json::to_vec(meta).unwrap())
    }

    /// Check the space id and token. `Err(reply)` is the response to send.
    fn authorize(&self, request: &Request, space: &str) -> io::Result<Result<SpaceMeta, Reply>> {
        if !is_hex(space, 16, 64) {
            return Ok(Err(error(400, "bad_request")));
        }
        let Some(token) = Self::bearer(request) else { return Ok(Err(error(401, "unauthorized"))) };
        match self.load_meta(space)? {
            Some(meta)
                if constant_time_eq(
                    meta.token_sha256.as_bytes(),
                    Self::token_hash(&token).as_bytes(),
                ) =>
            {
                Ok(Ok(meta))
            }
            Some(_) => Ok(Err(error(401, "unauthorized"))),
            None => Ok(Err(error(404, "not_found"))),
        }
    }

    fn register(&self, request: &Request, space: &str) -> io::Result<Reply> {
        if !is_hex(space, 16, 64) {
            return Ok(error(400, "bad_request"));
        }
        let Some(token) = Self::bearer(request) else { return Ok(error(401, "unauthorized")) };
        if self.load_meta(space)?.is_none() {
            fs::create_dir_all(self.space_dir(space))?;
            self.save_meta(
                space,
                &SpaceMeta {
                    token_sha256: Self::token_hash(&token),
                    next_seq: 1,
                    created_unix: now_unix(),
                },
            )?;
            return Ok(json(200, &serde_json::json!({ "created": true })));
        }
        Ok(match self.authorize(request, space)? {
            Ok(_) => json(200, &serde_json::json!({ "created": false })),
            Err(reply) => reply,
        })
    }

    /// Stored blobs of a space, oldest first, after removing expired ones.
    fn packages(&self, space: &str) -> io::Result<Vec<PackageInfo>> {
        let dir = self.space_dir(space);
        let cutoff = now_unix().saturating_sub(self.cfg.retention.as_secs());
        let mut out = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json")
                || path.file_name().and_then(|n| n.to_str()) == Some("space.json")
            {
                continue;
            }
            let info: PackageInfo =
                serde_json::from_slice(&fs::read(&path)?).map_err(io::Error::other)?;
            if info.created_unix < cutoff {
                let _ = fs::remove_file(dir.join(format!("{}.bin", info.seq)));
                let _ = fs::remove_file(&path);
                continue;
            }
            out.push(info);
        }
        out.sort_by_key(|p| p.seq);
        Ok(out)
    }

    fn upload(&self, request: &mut Request, space: &str) -> io::Result<Reply> {
        let mut meta = match self.authorize(request, space)? {
            Ok(meta) => meta,
            Err(reply) => return Ok(reply),
        };
        let device = request
            .headers()
            .iter()
            .find(|h| h.field.equiv("X-Doki-Device"))
            .map(|h| h.value.as_str().to_owned())
            .filter(|d| is_hex(d, 8, 64));
        let Some(device) = device else { return Ok(error(400, "bad_request")) };
        if request.body_length().is_some_and(|n| n as u64 > self.cfg.max_package_bytes) {
            return Ok(error(413, "too_large"));
        }
        let mut body = Vec::new();
        request.as_reader().take(self.cfg.max_package_bytes + 1).read_to_end(&mut body)?;
        if body.len() as u64 > self.cfg.max_package_bytes {
            return Ok(error(413, "too_large"));
        }
        let used: u64 = self.packages(space)?.iter().map(|p| p.size).sum();
        if used + body.len() as u64 > self.cfg.max_space_bytes {
            return Ok(error(507, "quota_exceeded"));
        }
        let seq = meta.next_seq;
        let dir = self.space_dir(space);
        write_atomic(&dir.join(format!("{seq}.bin")), &body)?;
        let info = PackageInfo {
            seq,
            from_device: device,
            size: body.len() as u64,
            created_unix: now_unix(),
        };
        // Metadata last: a blob without metadata is invisible and harmless.
        write_atomic(&dir.join(format!("{seq}.json")), &serde_json::to_vec(&info).unwrap())?;
        meta.next_seq += 1;
        self.save_meta(space, &meta)?;
        Ok(json(200, &serde_json::json!({ "seq": seq })))
    }

    fn list(&self, request: &Request, space: &str, after: u64) -> io::Result<Reply> {
        if let Err(reply) = self.authorize(request, space)? {
            return Ok(reply);
        }
        let list: Vec<PackageInfo> =
            self.packages(space)?.into_iter().filter(|p| p.seq > after).collect();
        Ok(json(200, &list))
    }

    fn download(&self, request: &Request, space: &str, seq: u64) -> io::Result<Reply> {
        if let Err(reply) = self.authorize(request, space)? {
            return Ok(reply);
        }
        if !self.packages(space)?.iter().any(|p| p.seq == seq) {
            return Ok(error(404, "not_found"));
        }
        let bytes = fs::read(self.space_dir(space).join(format!("{seq}.bin")))?;
        Ok(Response::from_data(bytes)
            .with_header(Header::from_bytes("Content-Type", "application/octet-stream").unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let relay = Relay::bind(RelayConfig::new(dir.path()), "127.0.0.1:0").unwrap();
        let url = format!("http://{}", relay.local_addr());
        std::thread::spawn(move || relay.run());
        (dir, url)
    }

    const SPACE: &str = "00112233445566778899aabbccddeeff";

    fn token(c: char) -> String {
        std::iter::repeat_n(c, 64).collect()
    }

    fn status(r: Result<ureq::Response, ureq::Error>) -> u16 {
        match r {
            Ok(r) => r.status(),
            Err(ureq::Error::Status(code, _)) => code,
            Err(e) => panic!("{e}"),
        }
    }

    #[test]
    fn store_list_fetch_and_reject_wrong_token() {
        let (_dir, url) = start();
        let space = format!("{url}/v1/spaces/{SPACE}");
        let auth = format!("Bearer {}", token('a'));
        let wrong = format!("Bearer {}", token('b'));

        assert_eq!(status(ureq::put(&space).set("Authorization", &auth).call()), 200);
        assert_eq!(status(ureq::put(&space).set("Authorization", &auth).call()), 200);
        assert_eq!(status(ureq::put(&space).set("Authorization", &wrong).call()), 401);

        let up = ureq::post(&format!("{space}/packages"))
            .set("Authorization", &auth)
            .set("X-Doki-Device", "deadbeefdeadbeef")
            .send_bytes(b"opaque");
        assert_eq!(status(up), 200);
        let denied = ureq::post(&format!("{space}/packages"))
            .set("Authorization", &wrong)
            .set("X-Doki-Device", "deadbeefdeadbeef")
            .send_bytes(b"evil");
        assert_eq!(status(denied), 401);

        let list: Vec<PackageInfo> = serde_json::from_reader(
            ureq::get(&format!("{space}/packages?after=0"))
                .set("Authorization", &auth)
                .call()
                .unwrap()
                .into_reader(),
        )
        .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].from_device, "deadbeefdeadbeef");
        let mut body = Vec::new();
        ureq::get(&format!("{space}/packages/{}", list[0].seq))
            .set("Authorization", &auth)
            .call()
            .unwrap()
            .into_reader()
            .read_to_end(&mut body)
            .unwrap();
        assert_eq!(body, b"opaque");

        let none: Vec<PackageInfo> = serde_json::from_reader(
            ureq::get(&format!("{space}/packages?after=1"))
                .set("Authorization", &auth)
                .call()
                .unwrap()
                .into_reader(),
        )
        .unwrap();
        assert!(none.is_empty());
        assert_eq!(
            status(ureq::get(&format!("{space}/packages/1")).set("Authorization", &wrong).call()),
            401
        );
        assert_eq!(status(ureq::get(&format!("{url}/v1/spaces/nothex/packages")).call()), 400);
    }

    #[test]
    fn expired_blobs_disappear_and_limits_hold() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = RelayConfig::new(dir.path());
        cfg.max_package_bytes = 10;
        cfg.retention = Duration::from_secs(0);
        let relay = Relay::bind(cfg, "127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1/spaces/{SPACE}", relay.local_addr());
        std::thread::spawn(move || relay.run());
        let auth = format!("Bearer {}", token('c'));
        ureq::put(&url).set("Authorization", &auth).call().unwrap();
        let big = ureq::post(&format!("{url}/packages"))
            .set("Authorization", &auth)
            .set("X-Doki-Device", "deadbeefdeadbeef")
            .send_bytes(&[0u8; 11]);
        assert_eq!(status(big), 413);
        ureq::post(&format!("{url}/packages"))
            .set("Authorization", &auth)
            .set("X-Doki-Device", "deadbeefdeadbeef")
            .send_bytes(b"small")
            .unwrap();
        std::thread::sleep(Duration::from_millis(1100));
        let list: Vec<PackageInfo> = serde_json::from_reader(
            ureq::get(&format!("{url}/packages"))
                .set("Authorization", &auth)
                .call()
                .unwrap()
                .into_reader(),
        )
        .unwrap();
        assert!(list.is_empty(), "retention 0 expires immediately");
    }
}
