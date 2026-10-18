//! Peer-to-peer sync between paired devices, with no server to host.
//!
//! Every device is equal. Each has its own key pair (an iroh endpoint id),
//! kept in its vault. Devices pair once with a code (shown as a QR code by
//! the app) and from then on any two of them sync directly whenever they can
//! reach each other: on the same network directly, across the internet
//! through NAT hole punching, falling back to public relays that forward
//! encrypted traffic they cannot read. Connections are QUIC with TLS bound to
//! both endpoint ids, so a device only ever talks to keys it trusts.
//!
//! # Trust
//!
//! A device trusts the peers in its [`PeerBook`]. Pairing adds the joining
//! device to the inviter's book and gives the joiner the inviter's book, so a
//! new device trusts the whole group at once. Books are exchanged on every
//! sync: a peer introduced by a trusted peer becomes trusted, and a peer one
//! device removed is removed everywhere the news reaches. Removals are kept
//! with their time so a removed device is never re-added from someone else's
//! book; only pairing it again (a newer `added_at`) brings it back, and that
//! news spreads the same way. Because each device has its own key,
//! removing a lost device needs no re-pairing of the others.
//!
//! # Sync session
//!
//! One bidirectional stream per connection, both sides running the same
//! steps at the same time:
//!
//! 1. Send a `Hello`: name, book, and the checkpoint ids held for each game.
//! 2. Merge the other side's book.
//! 3. Send a handoff package with every game whose current save the other
//!    side does not hold yet. Which save wins is never decided here: the
//!    receiving engine compares ancestry when the user reviews it.
//! 4. Import the other side's package. Importing only writes the vault:
//!    received progress waits in Incoming and is never installed here.
//!
//! Framing: JSON messages as `u32` little-endian length + bytes; the package
//! as `u64` length + bytes (length 0 when there is nothing to send).
//!
//! Sync never opens a save folder, so it runs against the vault alone and
//! can answer other devices in the background while the app is open.

use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::Engine as _;
use doki_core::error::StorageSide;
use doki_core::{BundleId, CheckpointId, Error, Result};
use doki_engine::{Engine, ImportSummary};
use doki_vault::{now_rfc3339, random_hex, Vault};
use iroh::endpoint::{presets, Connection, RecvStream, SendStream};
use iroh::protocol::{AcceptError, ProtocolHandler, Router};
use iroh::{Endpoint, EndpointAddr, PublicKey, RelayMode, RelayUrl, SecretKey, TransportAddr};
use serde::{Deserialize, Serialize};

const ALPN_PAIR: &[u8] = b"doki/pair/1";
const ALPN_SYNC: &[u8] = b"doki/sync/1";
const RECORD_KIND: &str = "peer";
const CODE_PREFIX: &str = "DOKIP1:";
const MAX_MESSAGE: u32 = 16 * 1024 * 1024;
const MAX_PACKAGE: u64 = 16 * 1024 * 1024 * 1024;
const INVITE_LIFETIME: Duration = Duration::from_secs(15 * 60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const SESSION_TIMEOUT: Duration = Duration::from_secs(30 * 60);
/// UDP port where a device showing a code answers nearby devices.
pub const LAN_PORT: u16 = 47_100;
const LAN_QUERY: &[u8] = b"DOKI-FIND1";
/// Wrong pairing attempts before every open code is withdrawn.
const MAX_PAIR_FAILURES: u32 = 20;
/// Letters and digits that cannot be confused when typed.
const SHORT_ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";

/// How the node reaches other devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    /// Direct connections, hole punching, public address lookup and relays.
    Internet,
    /// Loopback only, no lookup or relays. For tests.
    Loopback,
}

/// Where a device was last seen. Used before falling back to lookup by id.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerAddr {
    #[serde(default)]
    pub ips: Vec<String>,
    #[serde(default)]
    pub relay: Option<String>,
}

/// A trusted device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Endpoint id (public key).
    pub id: String,
    /// Name the device gave itself.
    pub name: String,
    #[serde(default)]
    pub addr: PeerAddr,
    /// When the device was paired, as recorded by the device that paired
    /// it. Spread with the book, so a newer pairing outranks an older removal.
    pub added_at: String,
    /// The trusted device that introduced this one, if not paired directly.
    #[serde(default)]
    pub introduced_by: Option<String>,
    /// Last completed sync with this device, as seen by this device.
    #[serde(default)]
    pub last_sync: Option<String>,
}

/// The devices this device trusts, and those removed from the group.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerBook {
    pub peers: BTreeMap<String, PeerInfo>,
    /// Removed devices and when they were removed. Kept so a removed device
    /// is never re-added from another book, unless it is paired again later.
    pub removed: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize)]
struct Identity {
    secret: String,
}

#[derive(Serialize, Deserialize)]
struct Invite {
    id: String,
    #[serde(default)]
    addr: PeerAddr,
    token: String,
}

#[derive(Serialize, Deserialize)]
struct PairRequest {
    token: String,
    name: String,
    addr: PeerAddr,
}

#[derive(Serialize, Deserialize)]
struct PairReply {
    accepted: bool,
    name: String,
    addr: PeerAddr,
    book: PeerBook,
}

#[derive(Serialize, Deserialize)]
struct Hello {
    name: String,
    addr: PeerAddr,
    book: PeerBook,
    /// Every checkpoint this device holds, by game.
    have: BTreeMap<BundleId, BTreeSet<CheckpointId>>,
}

/// Result of syncing with one device.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SessionReport {
    pub peer: String,
    pub peer_name: String,
    /// Games whose current save was sent.
    pub sent: usize,
    pub received: Option<ImportSummary>,
}

/// Result of [`PeerNode::sync_all`].
#[derive(Debug, Clone, Default, Serialize)]
pub struct SyncAllReport {
    pub sessions: Vec<SessionReport>,
    /// Devices that could not be reached, with the reason.
    pub unreachable: Vec<(String, String)>,
}

fn io_err(context: &str, e: std::io::Error) -> Error {
    Error::io(context, StorageSide::Vault, e)
}

fn net(what: &str, e: impl std::fmt::Display) -> Error {
    Error::SourceUnavailable(format!("{what}: {e}"))
}

fn peer_key(id: &str) -> Result<PublicKey> {
    PublicKey::from_str(id).map_err(|_| Error::InvalidInput(format!("not a device id: {id}")))
}

fn endpoint_addr(id: &str, addr: &PeerAddr) -> Result<EndpointAddr> {
    let mut parts: Vec<TransportAddr> = addr
        .ips
        .iter()
        .filter_map(|s| SocketAddr::from_str(s).ok())
        .map(TransportAddr::Ip)
        .collect();
    if let Some(relay) = addr.relay.as_deref().and_then(|r| RelayUrl::from_str(r).ok()) {
        parts.push(TransportAddr::Relay(relay));
    }
    Ok(EndpointAddr::from_parts(peer_key(id)?, parts))
}

/// Secrets compared without early exit.
fn same_secret(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn load_identity(vault: &Vault) -> Result<SecretKey> {
    if let Some(id) = vault.get_record::<Identity>(RECORD_KIND, "identity")? {
        let bytes: [u8; 32] = hex::decode(&id.secret)
            .ok()
            .and_then(|b| b.try_into().ok())
            .ok_or_else(|| Error::IntegrityFailed("device key record is damaged".into()))?;
        return Ok(SecretKey::from_bytes(&bytes));
    }
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("system random source available");
    vault.put_record(RECORD_KIND, "identity", &Identity { secret: hex::encode(bytes) })?;
    Ok(SecretKey::from_bytes(&bytes))
}

/// State shared by the node's API and its background handlers.
#[derive(Debug)]
struct Ctx {
    vault: PathBuf,
    me: String,
    endpoint: Endpoint,
    network: Network,
    /// Serializes read-modify-write of the peer book within this process.
    book_lock: Mutex<()>,
    /// Open pairing secrets (the short code, normalized) and when made.
    invites: Mutex<Vec<(String, Instant)>>,
    pair_failures: Mutex<u32>,
    lan_port: u16,
    lan_responder: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl Ctx {
    fn engine(&self) -> Result<Engine> {
        Engine::open(&self.vault)
    }

    fn book(&self) -> Result<PeerBook> {
        Ok(self.engine()?.vault().get_record(RECORD_KIND, "book")?.unwrap_or_default())
    }

    fn update_book<T>(&self, f: impl FnOnce(&mut PeerBook) -> T) -> Result<T> {
        let _guard = self.book_lock.lock().unwrap_or_else(|p| p.into_inner());
        let engine = self.engine()?;
        let mut book: PeerBook =
            engine.vault().get_record(RECORD_KIND, "book")?.unwrap_or_default();
        let out = f(&mut book);
        engine.vault().put_record(RECORD_KIND, "book", &book)?;
        Ok(out)
    }

    fn my_addr(&self) -> PeerAddr {
        let addr = self.endpoint.addr();
        let mut ips: Vec<String> = addr.ip_addrs().map(|a| a.to_string()).collect();
        if ips.is_empty() {
            ips = self.endpoint.bound_sockets().iter().map(|a| a.to_string()).collect();
        }
        let relay = addr.relay_urls().next().map(|r| r.to_string());
        PeerAddr { ips, relay }
    }

    fn trusts(&self, id: &str) -> Result<bool> {
        let book = self.book()?;
        Ok(book.peers.contains_key(id) && !book.removed.contains_key(id))
    }

    /// Fold in what a trusted peer knows: itself, the peers it introduces
    /// and the devices it removed.
    fn merge(&self, from: &str, name: &str, addr: &PeerAddr, theirs: &PeerBook) -> Result<()> {
        let me = self.me.clone();
        self.update_book(|book| {
            for (id, at) in &theirs.removed {
                let paired_since = book.peers.get(id).is_some_and(|p| p.added_at > *at);
                if *id == me || paired_since {
                    continue;
                }
                book.peers.remove(id);
                let known = book.removed.entry(id.clone()).or_insert_with(|| at.clone());
                if *known < *at {
                    *known = at.clone();
                }
            }
            if let Some(p) = book.peers.get_mut(from) {
                p.name = name.to_owned();
                if !addr.ips.is_empty() || addr.relay.is_some() {
                    p.addr = addr.clone();
                }
            }
            for (id, info) in &theirs.peers {
                let removed_since = book.removed.get(id).is_some_and(|at| *at >= info.added_at);
                if *id == me || removed_since || book.peers.contains_key(id) {
                    continue;
                }
                if peer_key(id).is_err() {
                    continue;
                }
                book.removed.remove(id);
                book.peers.insert(
                    id.clone(),
                    PeerInfo {
                        id: id.clone(),
                        name: info.name.clone(),
                        addr: info.addr.clone(),
                        added_at: info.added_at.clone(),
                        introduced_by: Some(from.to_owned()),
                        last_sync: None,
                    },
                );
            }
        })
    }

    fn hello(&self) -> Result<Hello> {
        let engine = self.engine()?;
        let mut have: BTreeMap<BundleId, BTreeSet<CheckpointId>> = BTreeMap::new();
        for (id, cp) in engine.vault().checkpoints()? {
            have.entry(cp.bundle_id).or_default().insert(id);
        }
        Ok(Hello {
            name: engine.vault().device_name().to_owned(),
            addr: self.my_addr(),
            book: self.book()?,
            have,
        })
    }

    /// Package of every game whose current save the other side lacks, or
    /// empty. Packages carry full history, so one current save brings
    /// everything it descends from.
    fn outgoing(
        &self,
        theirs: &BTreeMap<BundleId, BTreeSet<CheckpointId>>,
    ) -> Result<(usize, Vec<u8>)> {
        let engine = self.engine()?;
        let vault = engine.vault();
        let send: Vec<BundleId> = engine
            .bundles()?
            .into_iter()
            .filter(|b| match &b.head {
                Some(head) => !theirs.get(&b.bundle_id).is_some_and(|have| have.contains(head)),
                None => false,
            })
            .map(|b| b.bundle_id)
            .collect();
        if send.is_empty() {
            return Ok((0, Vec::new()));
        }
        let tmp = vault.root().join("tmp").join(format!("{}.dokipkg", random_hex(8)));
        let result = engine
            .export_package(&send, &tmp)
            .and_then(|_| std::fs::read(&tmp).map_err(|e| io_err("reading package", e)));
        let _ = std::fs::remove_file(&tmp);
        Ok((send.len(), result?))
    }

    fn import(&self, bytes: &[u8]) -> Result<ImportSummary> {
        let engine = self.engine()?;
        let tmp = engine.vault().root().join("tmp").join(format!("{}.dokipkg", random_hex(8)));
        let result = std::fs::write(&tmp, bytes)
            .map_err(|e| io_err("writing package", e))
            .and_then(|_| engine.import_package(&tmp));
        let _ = std::fs::remove_file(&tmp);
        result
    }

    fn mark_synced(&self, id: &str) -> Result<()> {
        self.update_book(|book| {
            if let Some(p) = book.peers.get_mut(id) {
                p.last_sync = Some(now_rfc3339());
            }
        })
    }
}

async fn write_msg<T: Serialize>(send: &mut SendStream, msg: &T) -> Result<()> {
    let bytes = serde_json::to_vec(msg).expect("messages serialize");
    send.write_all(&(bytes.len() as u32).to_le_bytes()).await.map_err(|e| net("sending", e))?;
    send.write_all(&bytes).await.map_err(|e| net("sending", e))
}

async fn read_msg<T: for<'de> Deserialize<'de>>(recv: &mut RecvStream) -> Result<T> {
    let mut len = [0u8; 4];
    recv.read_exact(&mut len).await.map_err(|e| net("receiving", e))?;
    let len = u32::from_le_bytes(len);
    if len > MAX_MESSAGE {
        return Err(Error::InvalidInput("message from the other device is too large".into()));
    }
    let mut buf = vec![0u8; len as usize];
    recv.read_exact(&mut buf).await.map_err(|e| net("receiving", e))?;
    serde_json::from_slice(&buf).map_err(|e| {
        Error::InvalidInput(format!("the other device sent something unexpected: {e}"))
    })
}

async fn write_package(send: &mut SendStream, bytes: &[u8]) -> Result<()> {
    send.write_all(&(bytes.len() as u64).to_le_bytes()).await.map_err(|e| net("sending", e))?;
    send.write_all(bytes).await.map_err(|e| net("sending", e))?;
    send.finish().map_err(|e| net("sending", e))
}

async fn read_package(recv: &mut RecvStream) -> Result<Vec<u8>> {
    let mut len = [0u8; 8];
    recv.read_exact(&mut len).await.map_err(|e| net("receiving", e))?;
    let len = u64::from_le_bytes(len);
    if len > MAX_PACKAGE {
        return Err(Error::InvalidInput("package from the other device is too large".into()));
    }
    let mut buf = vec![0u8; len as usize];
    recv.read_exact(&mut buf).await.map_err(|e| net("receiving", e))?;
    Ok(buf)
}

/// Run a blocking vault operation off the async workers.
async fn blocking<T: Send + 'static>(
    ctx: &Arc<Ctx>,
    f: impl FnOnce(&Ctx) -> Result<T> + Send + 'static,
) -> Result<T> {
    let ctx = ctx.clone();
    tokio::task::spawn_blocking(move || f(&ctx)).await.map_err(|_| Error::Io {
        context: "sync worker".into(),
        source: std::io::Error::other("stopped unexpectedly"),
    })?
}

/// Steps 1–4 of a sync session, identical on both sides.
async fn session(
    ctx: &Arc<Ctx>,
    peer: String,
    mut send: SendStream,
    mut recv: RecvStream,
) -> Result<SessionReport> {
    let mine = blocking(ctx, |c| c.hello()).await?;
    let (sent, theirs) = tokio::join!(write_msg(&mut send, &mine), read_msg::<Hello>(&mut recv));
    sent?;
    let theirs = theirs?;
    if theirs.book.removed.contains_key(&ctx.me) {
        return Err(Error::PermissionLost(format!(
            "{} has removed this device from the group",
            theirs.name
        )));
    }
    let (p, name, addr, book) = (peer.clone(), theirs.name.clone(), theirs.addr, theirs.book);
    blocking(ctx, move |c| c.merge(&p, &name, &addr, &book)).await?;

    let have = theirs.have;
    let (count, package) = blocking(ctx, move |c| c.outgoing(&have)).await?;
    let (sent, received) =
        tokio::join!(write_package(&mut send, &package), read_package(&mut recv));
    sent?;
    let received = received?;
    let summary = if received.is_empty() {
        None
    } else {
        Some(blocking(ctx, move |c| c.import(&received)).await?)
    };
    let p = peer.clone();
    blocking(ctx, move |c| c.mark_synced(&p)).await?;
    Ok(SessionReport { peer, peer_name: theirs.name, sent: count, received: summary })
}

#[derive(Debug, Clone)]
struct SyncHandler(Arc<Ctx>);

impl ProtocolHandler for SyncHandler {
    async fn accept(&self, connection: Connection) -> std::result::Result<(), AcceptError> {
        let peer = connection.remote_id().to_string();
        let ctx = self.0.clone();
        let p = peer.clone();
        let trusted = blocking(&ctx, move |c| c.trusts(&p)).await.unwrap_or(false);
        if !trusted {
            connection.close(1u32.into(), b"not paired");
            return Ok(());
        }
        let (send, recv) = connection.accept_bi().await.map_err(AcceptError::from_err)?;
        let _ = tokio::time::timeout(SESSION_TIMEOUT, session(&ctx, peer, send, recv)).await;
        connection.closed().await;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct PairHandler(Arc<Ctx>);

impl ProtocolHandler for PairHandler {
    async fn accept(&self, connection: Connection) -> std::result::Result<(), AcceptError> {
        let joiner = connection.remote_id().to_string();
        let (mut send, mut recv) = connection.accept_bi().await.map_err(AcceptError::from_err)?;
        let ctx = self.0.clone();
        let result: Result<()> = async {
            let req: PairRequest = read_msg(&mut recv).await?;
            let valid = {
                let mut invites = ctx.invites.lock().unwrap_or_else(|p| p.into_inner());
                invites.retain(|(_, at)| at.elapsed() < INVITE_LIFETIME);
                let valid = invites.iter().any(|(t, _)| same_secret(t, &req.token));
                if !valid {
                    let mut failures = ctx.pair_failures.lock().unwrap_or_else(|p| p.into_inner());
                    *failures += 1;
                    if *failures >= MAX_PAIR_FAILURES {
                        // Someone may be guessing: withdraw every open code.
                        invites.clear();
                        *failures = 0;
                    }
                }
                valid
            };
            let (me_name, book) = if valid {
                let j = joiner.clone();
                let (name, addr) = (req.name.clone(), req.addr.clone());
                blocking(&ctx, move |c| {
                    c.update_book(|b| {
                        b.removed.remove(&j);
                        b.peers.insert(
                            j.clone(),
                            PeerInfo {
                                id: j.clone(),
                                name,
                                addr,
                                added_at: now_rfc3339(),
                                introduced_by: None,
                                last_sync: None,
                            },
                        );
                    })?;
                    Ok((c.engine()?.vault().device_name().to_owned(), c.book()?))
                })
                .await?
            } else {
                (String::new(), PeerBook::default())
            };
            let reply = PairReply { accepted: valid, name: me_name, addr: ctx.my_addr(), book };
            write_msg(&mut send, &reply).await?;
            send.finish().map_err(|e| net("sending", e))?;
            Ok(())
        }
        .await;
        if result.is_ok() {
            connection.closed().await;
        }
        Ok(())
    }
}

/// A running peer: answers other devices in the background and syncs on
/// request. One per vault per process.
#[derive(Debug)]
pub struct PeerNode {
    rt: tokio::runtime::Runtime,
    router: Router,
    ctx: Arc<Ctx>,
}

/// Codes from [`PeerNode::invite`].
#[derive(Debug, Clone, Serialize)]
pub struct Invitation {
    /// Works from anywhere; shown as a QR code or copied.
    pub code: String,
    /// `XXXX-XXXX`, easy to type; works on the same network only.
    pub short: String,
}

/// One device in [`PeerNode::peers`].
#[derive(Debug, Clone, Serialize)]
pub struct PeerView {
    pub id: String,
    pub name: String,
    pub introduced_by: Option<String>,
    pub last_sync: Option<String>,
}

impl PeerNode {
    /// Start the node for the vault at `vault`, creating this device's key on
    /// first use.
    pub fn start(vault: impl AsRef<Path>, network: Network) -> Result<Self> {
        Self::start_on(vault, network, LAN_PORT)
    }

    /// [`PeerNode::start`] with another port for finding nearby devices.
    pub fn start_on(vault: impl AsRef<Path>, network: Network, lan_port: u16) -> Result<Self> {
        let vault = vault.as_ref().to_path_buf();
        let secret = load_identity(Engine::open(&vault)?.vault())?;
        let me = secret.public().to_string();
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("doki-peer")
            .build()
            .map_err(|e| io_err("starting sync", e))?;
        let endpoint = rt.block_on(async {
            let builder = match network {
                Network::Internet => Endpoint::builder(presets::N0),
                Network::Loopback => Endpoint::builder(presets::Minimal)
                    .relay_mode(RelayMode::Disabled)
                    .clear_address_lookup()
                    .bind_addr("127.0.0.1:0")
                    .map_err(|e| net("binding", e))?,
            };
            builder
                .secret_key(secret)
                .alpns(vec![ALPN_PAIR.to_vec(), ALPN_SYNC.to_vec()])
                .bind()
                .await
                .map_err(|e| net("starting sync", e))
        })?;
        let ctx = Arc::new(Ctx {
            vault,
            me,
            endpoint: endpoint.clone(),
            network,
            book_lock: Mutex::new(()),
            invites: Mutex::new(Vec::new()),
            pair_failures: Mutex::new(0),
            lan_port,
            lan_responder: Mutex::new(None),
        });
        let router = rt.block_on(async {
            Router::builder(endpoint)
                .accept(ALPN_PAIR, PairHandler(ctx.clone()))
                .accept(ALPN_SYNC, SyncHandler(ctx.clone()))
                .spawn()
        });
        Ok(PeerNode { rt, router, ctx })
    }

    /// This device's id.
    pub fn id(&self) -> &str {
        &self.ctx.me
    }

    /// Open pairing for 15 minutes, for any number of devices. Anyone with
    /// either code can join the group while it is open, so show them only to
    /// your own devices.
    pub fn invite(&self) -> Result<Invitation> {
        if self.ctx.network == Network::Internet {
            // Wait briefly for a relay and public addresses so the code works
            // from other networks too; local addresses are enough on Wi-Fi.
            let ep = self.ctx.endpoint.clone();
            self.rt.block_on(async move {
                let _ = tokio::time::timeout(Duration::from_secs(5), ep.online()).await;
            });
        }
        let mut raw = [0u8; 8];
        getrandom::fill(&mut raw).expect("system random source available");
        let short: String = raw
            .iter()
            .map(|b| SHORT_ALPHABET[*b as usize % SHORT_ALPHABET.len()] as char)
            .collect();
        self.ctx
            .invites
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push((short.clone(), Instant::now()));
        self.start_lan_responder();
        let invite =
            Invite { id: self.ctx.me.clone(), addr: self.ctx.my_addr(), token: short.clone() };
        let json = serde_json::to_vec(&invite).expect("invite serializes");
        Ok(Invitation {
            code: format!(
                "{CODE_PREFIX}{}",
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json)
            ),
            short: format!("{}-{}", &short[..4], &short[4..]),
        })
    }

    /// Answer nearby devices looking for someone showing a code. The answer
    /// is only this device's id and addresses, never the code: the joiner
    /// proves the code over the encrypted connection.
    fn start_lan_responder(&self) {
        let mut slot = self.ctx.lan_responder.lock().unwrap_or_else(|p| p.into_inner());
        if slot.as_ref().is_some_and(|h| !h.is_finished()) {
            return;
        }
        let ctx = self.ctx.clone();
        let bind = match ctx.network {
            Network::Internet => SocketAddr::from(([0, 0, 0, 0], ctx.lan_port)),
            Network::Loopback => SocketAddr::from(([127, 0, 0, 1], ctx.lan_port)),
        };
        *slot = Some(self.rt.spawn(async move {
            // Another app on the port only disables the short code; the long
            // code and QR code keep working.
            let Ok(socket) = tokio::net::UdpSocket::bind(bind).await else { return };
            let mut buf = [0u8; 64];
            loop {
                let Ok((n, from)) = socket.recv_from(&mut buf).await else { continue };
                let open = {
                    let mut invites = ctx.invites.lock().unwrap_or_else(|p| p.into_inner());
                    invites.retain(|(_, at)| at.elapsed() < INVITE_LIFETIME);
                    !invites.is_empty()
                };
                if open && &buf[..n] == LAN_QUERY {
                    let reply =
                        Invite { id: ctx.me.clone(), addr: ctx.my_addr(), token: String::new() };
                    let _ = socket
                        .send_to(&serde_json::to_vec(&reply).expect("serializes"), from)
                        .await;
                }
            }
        }));
    }

    /// Devices on this network that are showing a code right now.
    async fn find_nearby(&self) -> Vec<Invite> {
        let port = self.ctx.lan_port;
        let Ok(socket) = tokio::net::UdpSocket::bind(("0.0.0.0", 0)).await else {
            return Vec::new();
        };
        let _ = socket.set_broadcast(true);
        let targets = [
            SocketAddr::from(([255, 255, 255, 255], port)),
            SocketAddr::from(([127, 0, 0, 1], port)),
        ];
        let mut found: BTreeMap<String, Invite> = BTreeMap::new();
        let mut buf = vec![0u8; 8192];
        for _ in 0..4 {
            for t in targets {
                let _ = socket.send_to(LAN_QUERY, t).await;
            }
            let until = tokio::time::Instant::now() + Duration::from_millis(700);
            while let Ok(Ok((n, _))) =
                tokio::time::timeout_at(until, socket.recv_from(&mut buf)).await
            {
                if let Ok(inv) = serde_json::from_slice::<Invite>(&buf[..n]) {
                    if inv.id != self.ctx.me {
                        found.entry(inv.id.clone()).or_insert(inv);
                    }
                }
            }
            if !found.is_empty() {
                break;
            }
        }
        found.into_values().collect()
    }

    /// Join the group of the device showing `code` (the long code, or the
    /// short code on the same network), then sync with it. Returns the
    /// inviting device's name.
    pub fn join(&self, code: &str) -> Result<String> {
        let code = code.trim();
        if let Some(b64) = code.strip_prefix(CODE_PREFIX) {
            let invite: Invite = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(b64)
                .ok()
                .and_then(|j| serde_json::from_slice(&j).ok())
                .ok_or_else(|| Error::InvalidInput("that is not a Doki pairing code".into()))?;
            return self.pair_with(invite);
        }
        let short: String = code
            .chars()
            .filter(|c| !matches!(c, '-' | ' '))
            .collect::<String>()
            .to_ascii_uppercase();
        if short.len() != 8 || !short.bytes().all(|b| SHORT_ALPHABET.contains(&b)) {
            return Err(Error::InvalidInput("that is not a Doki pairing code".into()));
        }
        let nearby = self.rt.block_on(self.find_nearby());
        if nearby.is_empty() {
            return Err(Error::SourceUnavailable(
                "no device showing a code was found on this network; use the long code instead"
                    .into(),
            ));
        }
        let mut last = None;
        for mut invite in nearby {
            invite.token = short.clone();
            match self.pair_with(invite) {
                Ok(name) => return Ok(name),
                Err(e) => last = Some(e),
            }
        }
        Err(last.expect("at least one attempt"))
    }

    fn pair_with(&self, invite: Invite) -> Result<String> {
        if invite.id == self.ctx.me {
            return Err(Error::InvalidInput("that code is from this device".into()));
        }
        let addr = endpoint_addr(&invite.id, &invite.addr)?;
        let inviter = invite.id.clone();
        let ctx = self.ctx.clone();
        let name = self.rt.block_on(async move {
            let conn = tokio::time::timeout(CONNECT_TIMEOUT, ctx.endpoint.connect(addr, ALPN_PAIR))
                .await
                .map_err(|_| net("pairing", "the other device did not answer"))?
                .map_err(|e| net("pairing", e))?;
            let (mut send, mut recv) = conn.open_bi().await.map_err(|e| net("pairing", e))?;
            let my_name =
                blocking(&ctx, |c| Ok(c.engine()?.vault().device_name().to_owned())).await?;
            write_msg(
                &mut send,
                &PairRequest { token: invite.token, name: my_name, addr: ctx.my_addr() },
            )
            .await?;
            send.finish().map_err(|e| net("pairing", e))?;
            let reply: PairReply = read_msg(&mut recv).await?;
            conn.close(0u32.into(), b"paired");
            if !reply.accepted {
                return Err(Error::PermissionLost(
                    "the pairing code has expired or was not accepted; show a new one".into(),
                ));
            }
            let (id, name, raddr, book) =
                (invite.id.clone(), reply.name.clone(), reply.addr, reply.book);
            blocking(&ctx, move |c| {
                c.update_book(|b| {
                    b.removed.remove(&id);
                    b.peers.insert(
                        id.clone(),
                        PeerInfo {
                            id: id.clone(),
                            name: name.clone(),
                            addr: raddr.clone(),
                            added_at: now_rfc3339(),
                            introduced_by: None,
                            last_sync: None,
                        },
                    );
                })?;
                c.merge(&id, &name, &raddr, &book)
            })
            .await?;
            Ok::<_, Error>(reply.name)
        })?;
        // First exchange right away, so both devices see each other's games.
        self.sync_with(&inviter)?;
        Ok(name)
    }

    /// Sync with one trusted device.
    pub fn sync_with(&self, id: &str) -> Result<SessionReport> {
        let ctx = self.ctx.clone();
        let id = id.to_owned();
        self.rt.block_on(connect_and_sync(ctx, id))
    }

    /// Sync with every trusted device that can be reached now.
    pub fn sync_all(&self) -> Result<SyncAllReport> {
        let book = self.ctx.book()?;
        let ctx = self.ctx.clone();
        let ids: Vec<String> =
            book.peers.keys().filter(|id| !book.removed.contains_key(*id)).cloned().collect();
        let results = self.rt.block_on(async move {
            let tasks: Vec<_> = ids
                .into_iter()
                .map(|id| {
                    let ctx = ctx.clone();
                    tokio::spawn(async move { (id.clone(), connect_and_sync(ctx, id).await) })
                })
                .collect();
            let mut out = Vec::new();
            for t in tasks {
                if let Ok(r) = t.await {
                    out.push(r);
                }
            }
            out
        });
        let mut report = SyncAllReport::default();
        for (id, r) in results {
            match r {
                Ok(s) => report.sessions.push(s),
                Err(e) => {
                    let name = book.peers.get(&id).map(|p| p.name.clone()).unwrap_or(id);
                    report.unreachable.push((name, e.to_string()));
                }
            }
        }
        Ok(report)
    }

    /// Trusted devices, newest first.
    pub fn peers(&self) -> Result<Vec<PeerView>> {
        let book = self.ctx.book()?;
        let mut out: Vec<PeerView> = book
            .peers
            .values()
            .filter(|p| !book.removed.contains_key(&p.id))
            .map(|p| PeerView {
                id: p.id.clone(),
                name: p.name.clone(),
                introduced_by: p.introduced_by.as_ref().map(|i| {
                    book.peers.get(i).map(|x| x.name.clone()).unwrap_or_else(|| i.clone())
                }),
                last_sync: p.last_sync.clone(),
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// Remove a device from the group. It can no longer sync with this
    /// device, and the removal spreads to the others as they sync. Its
    /// checkpoints already received stay in history.
    pub fn remove(&self, id: &str) -> Result<()> {
        if id == self.ctx.me {
            return Err(Error::InvalidInput("this device cannot remove itself".into()));
        }
        peer_key(id)?;
        let id = id.to_owned();
        self.ctx.update_book(|b| {
            b.peers.remove(&id);
            b.removed.insert(id, now_rfc3339());
        })
    }

    /// Stop answering other devices.
    pub fn shutdown(self) {
        let PeerNode { rt, router, .. } = self;
        let _ = rt.block_on(router.shutdown());
    }
}

async fn connect_and_sync(ctx: Arc<Ctx>, id: String) -> Result<SessionReport> {
    let i = id.clone();
    let info = blocking(&ctx, move |c| {
        let book = c.book()?;
        if book.removed.contains_key(&i) {
            return Err(Error::InvalidInput("that device was removed".into()));
        }
        book.peers.get(&i).cloned().ok_or_else(|| Error::NotFound("not a paired device".into()))
    })
    .await?;
    let addr = endpoint_addr(&id, &info.addr)?;
    let conn = tokio::time::timeout(CONNECT_TIMEOUT, ctx.endpoint.connect(addr, ALPN_SYNC))
        .await
        .map_err(|_| net(&info.name, "not reachable right now"))?
        .map_err(|e| net(&info.name, e))?;
    let (send, recv) = conn.open_bi().await.map_err(|e| net(&info.name, e))?;
    let report = tokio::time::timeout(SESSION_TIMEOUT, session(&ctx, id, send, recv))
        .await
        .map_err(|_| net(&info.name, "sync took too long"))??;
    conn.close(0u32.into(), b"done");
    Ok(report)
}

#[cfg(test)]
mod tests;
