//! The capture-file reader and its preprocessing (Phase 3.5b plan §2 T1+T2).
//!
//! This module turns a relay NDJSON capture file into the one shape every
//! downstream consumer (H1's setup importer, H2's translator) reads: a
//! deduplicated, supersession-collapsed, move-ordered feed. It knows the WIRE
//! — envelopes, transports, packet identity — and nothing about Yokohama's
//! rules; card and move interpretation live in `yokohama-cli::bga`, which is
//! why this crate still depends on nothing beyond `std` + `serde_json`.
//!
//! The standing posture is HARD-ERROR-NEVER-GUESS (design §6): an envelope
//! shape this module does not recognize is a typed `FeedError` naming what was
//! seen, never a skip. The recognized shapes are pinned to what the H0 corpus
//! measured (`docs/plans/2026-08-08-bga-notification-inventory.md`); a BGA
//! drift therefore surfaces as a failing parse, not as silent corruption.
//!
//! Wire facts this module is built on, each measured from table #895724349:
//!
//! - **Delivery is at-least-once** (ledger N-4): every socket packet arrived
//!   twice (two sockets), history packets up to three times (overlapping
//!   fetches). Dedupe keys on PACKET identity — `packet_id` within a
//!   transport, with content equality asserted — never on any timestamp and
//!   never on `move_id` (a takeback reuses move ids).
//! - **Supersession is structural**: a `sequence` packet whose `prevpacket`
//!   is not `[packet_id - 1]` rewinds the line; every packet strictly between
//!   the two ids is superseded. The `undoRestorePoint` notification is
//!   REQUIRED corroboration, not the detector — a `move_id` regression test
//!   catches only 3 of the 5 observed undos (inventory §4.1: two undos
//!   superseded exactly one packet, so the replacement's move id EQUALS the
//!   previous arrival's rather than falling below it).
//! - **The history feed is already collapsed** (inventory §4.3): BGA serves
//!   only the surviving line, with `undoRestorePoint` retained as an inert
//!   marker occupying its own move slot.
//! - **The two feeds are two different data sources, not two deliveries of
//!   one** (inventory §5): the history copy of a notification is arg-thin,
//!   and history additionally omits WHOLE notifications of the kinds in
//!   [`SOCKET_ONLY_KINDS`]. The merge check therefore compares the history
//!   form against the socket form FILTERED to shared kinds, on `(kind, uid)`
//!   pairs — measured to hold for all 88 doubly-covered moves of the corpus.
//! - **Moveless packets exist and are chrome**: history serves null-`move_id`
//!   packets with EMPTY `data`; the socket serves the same packet ids with NO
//!   `move_id` key and a lone `wakeupPlayers` notification. Both are dropped
//!   and counted. A moveless packet carrying anything not in
//!   [`MOVELESS_CHROME_KINDS`] is an error — a game notification without a
//!   move id would otherwise vanish silently.
//! - **Scalar-type trap** (inventory §2): `packet_id`/`move_id` are JSON
//!   strings in history and integers on the socket. [`flex_u64`] normalizes
//!   at the parse boundary.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Notification kinds the history feed omits ENTIRELY (not merely thins).
/// Measured: filtering these from the socket form makes the `(kind, uid)`
/// sequence equal to the history form on 88 of 88 doubly-covered moves of the
/// H0 corpus; without the filter, 83 of 88 disagree.
pub const SOCKET_ONLY_KINDS: [&str; 5] = [
    "gameStateChange",
    "updateReflexionTime",
    "wakeupPlayers",
    "tableWindow",
    // The multi-active state update BGA emits during the SIMULTANEOUS opening
    // placement — measured live at the gate game (table 896153951, move 1).
    // The proving corpus never showed it because its socket joined mid-game,
    // past the multi-active opening. Framework chrome of the same class as
    // gameStateChange: socket-only, never in history.
    "gameStateMultipleActiveUpdate",
];

/// The only notification kinds a MOVELESS packet may carry (besides none).
/// Observed: socket packets 177/188/199/200/209 carry `wakeupPlayers` with no
/// `move_id` key; the history copies of the same ids have `move_id: null` and
/// empty `data`. Anything else on a moveless packet is a [`FeedError::MovelessGamePacket`].
pub const MOVELESS_CHROME_KINDS: [&str; 1] = ["wakeupPlayers"];

/// Typed reader/collapse errors. Every variant names what was actually seen —
/// these strings are the bridge's drift alarm, so they carry the offending
/// values rather than a generic complaint.
#[derive(Debug, Clone, PartialEq)]
pub enum FeedError {
    /// A capture line that is not valid JSON.
    MalformedLine { line: usize, detail: String },
    /// A capture line missing a required field.
    MissingField { line: usize, context: String, field: &'static str },
    /// `kind` was neither `snapshot` nor `packets`.
    UnknownKind { line: usize, kind: String },
    /// `payload.source` was not `xhr`/`fetch`/`websocket`.
    UnknownSource { line: usize, source: String },
    /// An xhr/fetch body that is not the `{status, data: {valid, data: […]}}` shape.
    MalformedHistoryBody { line: usize, detail: String },
    /// A websocket `body_text` frame that is not valid JSON.
    MalformedSocketFrame { line: usize, detail: String },
    /// A websocket frame outside the measured taxonomy (heartbeat / handshake
    /// reply / push pub|join|leave).
    UnrecognizedSocketFrame { line: usize, excerpt: String },
    /// A packet whose `packet_type` is not `history`/`sequence`/`single`.
    UnknownPacketType { line: usize, packet_type: String },
    /// A scalar that was neither an integer nor a numeric string.
    BadScalar { line: usize, context: String, value: String },
    /// A `sequence` packet whose `prevpacket` is not a single id below its own.
    UnexpectedPrevpacket { packet_id: u64, prevpacket: Vec<u64> },
    /// A moveless packet carrying a notification kind outside
    /// [`MOVELESS_CHROME_KINDS`] — a game event would be dropped silently.
    MovelessGamePacket { transport: Transport, packet_id: u64, kinds: Vec<String> },
    /// Two deliveries of one `packet_id` with DIFFERENT content — at-least-once
    /// delivery promises repeats, never contradictions.
    DuplicatePacketDiffers { transport: Transport, packet_id: u64 },
    /// A structural rewind (`prevpacket` skipping back) with no
    /// `undoRestorePoint` notification in the rewinding packet.
    StructuralBreakWithoutMarker { packet_id: u64, prevpacket: Vec<u64> },
    /// A `move_id` regression on the socket feed WITHOUT a structural
    /// `prevpacket` break — outside everything the corpus measured.
    RegressionWithoutStructuralBreak { packet_id: u64, move_id: u64, max_seen: u64 },
    /// Two surviving packets of one transport claiming the same `move_id`.
    DuplicateMoveId { transport: Transport, move_id: u64 },
    /// The capture holds packets for more than one table and no `--table`
    /// filter was given.
    MultipleTables { tables: Vec<u64> },
    /// The requested table has no packets in this capture.
    TableNotInCapture { table: u64, present: Vec<u64> },
    /// No packets at all (and no table filter to name the table).
    NoPackets,
    /// History and socket forms of one move disagree after socket-only
    /// filtering — the two feeds describe different games.
    FeedDisagreement {
        move_id: u64,
        history: Vec<(String, String)>,
        socket_filtered: Vec<(String, String)>,
    },
}

impl fmt::Display for FeedError {
    // Several arms interpolate verbatim wire strings (`kind`, `source`,
    // `packet_type`, `excerpt`, serde `detail`) with `{}`. This impl is NOT a
    // sanitization boundary — it is inside the relay crate, which never prints
    // a `FeedError` to a terminal. The one consumer that does is the importer,
    // which surfaces every `FeedError` wrapped in `ImportError::Feed`, whose
    // Display scrubs control bytes at that single choke point (see
    // `crate::sanitize`). A future terminal caller must sanitize likewise.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FeedError::MalformedLine { line, detail } => {
                write!(f, "capture line {line}: not valid JSON: {detail}")
            }
            FeedError::MissingField { line, context, field } => {
                write!(f, "capture line {line}: {context} is missing required field `{field}`")
            }
            FeedError::UnknownKind { line, kind } => {
                write!(f, "capture line {line}: unknown capture kind `{kind}` (expected snapshot|packets)")
            }
            FeedError::UnknownSource { line, source } => {
                write!(f, "capture line {line}: unknown payload source `{source}` (expected xhr|fetch|websocket)")
            }
            FeedError::MalformedHistoryBody { line, detail } => {
                write!(f, "capture line {line}: history body is not the {{status, data:{{valid, data:[…]}}}} shape: {detail}")
            }
            FeedError::MalformedSocketFrame { line, detail } => {
                write!(f, "capture line {line}: websocket frame is not valid JSON: {detail}")
            }
            FeedError::UnrecognizedSocketFrame { line, excerpt } => {
                write!(f, "capture line {line}: websocket frame outside the measured taxonomy: {excerpt}")
            }
            FeedError::UnknownPacketType { line, packet_type } => {
                write!(f, "capture line {line}: unknown packet_type `{packet_type}`")
            }
            FeedError::BadScalar { line, context, value } => {
                write!(f, "capture line {line}: {context}: `{value}` is neither an integer nor a numeric string")
            }
            FeedError::UnexpectedPrevpacket { packet_id, prevpacket } => {
                write!(f, "socket packet {packet_id}: prevpacket {prevpacket:?} is not a single earlier id — unmeasured shape")
            }
            FeedError::MovelessGamePacket { transport, packet_id, kinds } => {
                write!(f, "{transport:?} packet {packet_id} has no move_id but carries non-chrome notifications {kinds:?}")
            }
            FeedError::DuplicatePacketDiffers { transport, packet_id } => {
                write!(f, "{transport:?} packet {packet_id} was delivered twice with DIFFERENT content")
            }
            FeedError::StructuralBreakWithoutMarker { packet_id, prevpacket } => {
                write!(f, "socket packet {packet_id} rewinds (prevpacket {prevpacket:?}) but carries no undoRestorePoint marker")
            }
            FeedError::RegressionWithoutStructuralBreak { packet_id, move_id, max_seen } => {
                write!(f, "socket packet {packet_id} regresses move_id to {move_id} (max seen {max_seen}) without a prevpacket break")
            }
            FeedError::DuplicateMoveId { transport, move_id } => {
                write!(f, "two surviving {transport:?} packets claim move_id {move_id}")
            }
            FeedError::MultipleTables { tables } => {
                write!(f, "capture holds packets for more than one table ({tables:?}); pass a table filter")
            }
            FeedError::TableNotInCapture { table, present } => {
                write!(f, "table {table} has no packets in this capture (present: {present:?})")
            }
            FeedError::NoPackets => write!(f, "capture holds no packets"),
            FeedError::FeedDisagreement { move_id, history, socket_filtered } => {
                write!(
                    f,
                    "move {move_id}: history and socket forms disagree after socket-only filtering \
                     (history {history:?} vs socket {socket_filtered:?})"
                )
            }
        }
    }
}

impl std::error::Error for FeedError {}

/// Which feed a packet arrived on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// A `notificationHistory` response (xhr or fetch) — the catch-up feed,
    /// already supersession-collapsed by BGA, arg-THIN (inventory §5).
    History,
    /// A realtime `sequence` push — the live feed, arg-RICH, carrying the raw
    /// supersession signal.
    Socket,
}

/// One notification, envelope-parsed only: `args` stays a raw
/// [`serde_json::Value`] because interpretation belongs to the translator
/// (plan §2 T1: "the reader owns only the envelope").
#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub uid: String,
    /// BGA's `type` field (`kind` here; `type` is a Rust keyword).
    pub kind: String,
    pub log: Option<String>,
    pub args: Value,
}

/// One game-bearing packet, scalars normalized (`flex_u64`).
#[derive(Debug, Clone, PartialEq)]
pub struct Packet {
    pub table_id: u64,
    pub packet_id: u64,
    /// `None` for a moveless chrome packet (dropped and counted at collapse).
    pub move_id: Option<u64>,
    /// BGA's own packet timestamp (unix seconds), when present.
    pub time: Option<u64>,
    pub transport: Transport,
    /// Socket only: the predecessor claim, the supersession signal.
    pub prevpacket: Option<Vec<u64>>,
    pub notifications: Vec<Notification>,
}

impl Packet {
    /// Content identity for dedupe: everything except arrival bookkeeping.
    fn same_content(&self, other: &Packet) -> bool {
        self.move_id == other.move_id
            && self.time == other.time
            && self.prevpacket == other.prevpacket
            && self.notifications == other.notifications
    }
}

/// One `gamedatas` snapshot, kept raw for the setup importer.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub received_at_ms: u64,
    pub captured_at_ms: Option<u64>,
    pub href: Option<String>,
    pub gamedatas: Value,
}

/// Everything the reader saw that is NOT a game packet, counted rather than
/// dropped silently — an anti-vacuity surface (a capture of pure chrome parses
/// to zero packets and a tally that says why) and the home of the one piece of
/// identity chrome carries: the `/player/p<id>` channel names the capturing
/// session's own player (BGA subscribes a client only to its own channel).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChromeTally {
    pub heartbeat_frames: u32,
    /// Handshake replies: frames carrying a top-level `id` (connect/subscribe/presence).
    pub handshake_frames: u32,
    pub join_leave_frames: u32,
    /// `pub` frames on channels other than `/table/t<id>`.
    pub other_channel_pubs: u32,
    /// The non-table channels seen (e.g. `/chat/general`, `/player/p123`).
    pub other_channels: BTreeSet<String>,
    /// `/player/p<id>` channels seen — the own-seat evidence.
    pub player_channels: BTreeSet<String>,
    /// `single`-type packets (out-of-band chrome; no packet_id, no move_id —
    /// inventory §2: no `single` packet ever carried a game notification).
    pub single_packets: u32,
    pub single_kinds: BTreeMap<String, u32>,
}

/// The parsed capture file, before collapse: packets in ARRIVAL order
/// (supersession is an arrival-order operation), snapshots in file order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RawFeed {
    pub snapshots: Vec<Snapshot>,
    pub packets: Vec<Packet>,
    pub chrome: ChromeTally,
    /// Non-blank capture lines read.
    pub lines: usize,
}

/// One supersession event, for the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupersessionEvent {
    pub undo_packet_id: u64,
    pub undo_move_id: Option<u64>,
    pub superseded_packet_ids: Vec<u64>,
}

/// Collapse bookkeeping — every drop is counted, so "the dedupe never deduped"
/// is checkable (plan §2 T2's anti-vacuity pin).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FeedReport {
    pub history_packets_raw: u32,
    pub history_duplicates_dropped: u32,
    pub socket_packets_raw: u32,
    pub socket_duplicates_dropped: u32,
    pub superseded: Vec<SupersessionEvent>,
    /// Moveless chrome packets dropped, per transport.
    pub moveless_dropped_history: u32,
    pub moveless_dropped_socket: u32,
    pub moves: usize,
    /// Moves present in BOTH forms (socket-enriched).
    pub enriched: usize,
    /// Gaps in `1..=max(move_id)` — a capture whose history fetch missed the
    /// start of the game shows them here rather than pretending completeness.
    pub missing_move_ids: Vec<u64>,
    pub snapshots: usize,
    pub chrome: ChromeTally,
}

/// One move of the collapsed feed. `history` is the spine form (thin,
/// pre-collapsed by BGA); `socket` the enriched form where the live window
/// covered the move. Either may be absent — a socket-only live capture has no
/// spine, a pre-window move no enrichment.
#[derive(Debug, Clone, PartialEq)]
pub struct CollapsedMove {
    pub move_id: u64,
    pub history: Option<Packet>,
    pub socket: Option<Packet>,
}

/// The preprocessed feed every downstream consumer reads.
#[derive(Debug, Clone, PartialEq)]
pub struct CollapsedFeed {
    pub table_id: u64,
    /// In `move_id` order, one entry per surviving move.
    pub moves: Vec<CollapsedMove>,
    pub snapshots: Vec<Snapshot>,
    pub report: FeedReport,
}

impl CollapsedFeed {
    /// The earliest snapshot by `captured_at_ms` (file order breaks ties) —
    /// the setup importer's input.
    pub fn earliest_snapshot(&self) -> Option<&Snapshot> {
        self.snapshots
            .iter()
            .min_by_key(|s| s.captured_at_ms.unwrap_or(u64::MAX))
    }
}

/// Accept a JSON integer or a numeric string (the §2 scalar-type trap:
/// history serves `"124"`, the socket serves `124`).
pub fn flex_u64(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.parse::<u64>().ok(),
        _ => None,
    }
}

fn required_flex_u64(
    obj: &Value,
    field: &'static str,
    line: usize,
    context: &str,
) -> Result<u64, FeedError> {
    let v = obj.get(field).ok_or_else(|| FeedError::MissingField {
        line,
        context: context.to_string(),
        field,
    })?;
    flex_u64(v).ok_or_else(|| FeedError::BadScalar {
        line,
        context: format!("{context}.{field}"),
        value: v.to_string(),
    })
}

fn parse_notifications(data: &Value, line: usize, context: &str) -> Result<Vec<Notification>, FeedError> {
    let arr = data.as_array().ok_or_else(|| FeedError::MissingField {
        line,
        context: context.to_string(),
        field: "data",
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for n in arr {
        let uid = n
            .get("uid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FeedError::MissingField {
                line,
                context: format!("{context} notification"),
                field: "uid",
            })?
            .to_string();
        let kind = n
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FeedError::MissingField {
                line,
                context: format!("{context} notification"),
                field: "type",
            })?
            .to_string();
        let log = n.get("log").and_then(|v| v.as_str()).map(|s| s.to_string());
        let args = n.get("args").cloned().unwrap_or(Value::Null);
        out.push(Notification { uid, kind, log, args });
    }
    Ok(out)
}

/// Parse one packet object (either transport). `move_id` absent or null both
/// normalize to `None` — history spells the moveless case `null`, the socket
/// omits the key.
fn parse_packet(pk: &Value, transport: Transport, line: usize) -> Result<Packet, FeedError> {
    let context = match transport {
        Transport::History => "history packet",
        Transport::Socket => "socket packet",
    };
    let table_id = required_flex_u64(pk, "table_id", line, context)?;
    let packet_id = required_flex_u64(pk, "packet_id", line, context)?;
    let move_id = match pk.get("move_id") {
        None | Some(Value::Null) => None,
        Some(v) => Some(flex_u64(v).ok_or_else(|| FeedError::BadScalar {
            line,
            context: format!("{context}.move_id"),
            value: v.to_string(),
        })?),
    };
    let time = pk.get("time").and_then(flex_u64);
    let prevpacket = match transport {
        Transport::History => None,
        Transport::Socket => {
            let raw = pk.get("prevpacket").ok_or_else(|| FeedError::MissingField {
                line,
                context: context.to_string(),
                field: "prevpacket",
            })?;
            let arr = raw.as_array().ok_or_else(|| FeedError::BadScalar {
                line,
                context: format!("{context}.prevpacket"),
                value: raw.to_string(),
            })?;
            let mut ids = Vec::with_capacity(arr.len());
            for v in arr {
                ids.push(flex_u64(v).ok_or_else(|| FeedError::BadScalar {
                    line,
                    context: format!("{context}.prevpacket entry"),
                    value: v.to_string(),
                })?);
            }
            Some(ids)
        }
    };
    let notifications = parse_notifications(
        pk.get("data").unwrap_or(&Value::Null),
        line,
        context,
    )?;
    Ok(Packet {
        table_id,
        packet_id,
        move_id,
        time,
        transport,
        prevpacket,
        notifications,
    })
}

fn parse_history_body(body_text: &str, line: usize, feed: &mut RawFeed) -> Result<(), FeedError> {
    let body: Value = serde_json::from_str(body_text).map_err(|e| FeedError::MalformedHistoryBody {
        line,
        detail: e.to_string(),
    })?;
    if body.get("status").is_none() {
        return Err(FeedError::MalformedHistoryBody {
            line,
            detail: "missing `status`".to_string(),
        });
    }
    let packets = body
        .get("data")
        .and_then(|d| d.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| FeedError::MalformedHistoryBody {
            line,
            detail: "missing `data.data` array".to_string(),
        })?;
    for pk in packets {
        let ptype = pk.get("packet_type").and_then(|v| v.as_str()).unwrap_or("<absent>");
        if ptype != "history" {
            return Err(FeedError::UnknownPacketType {
                line,
                packet_type: ptype.to_string(),
            });
        }
        feed.packets.push(parse_packet(pk, Transport::History, line)?);
    }
    Ok(())
}

fn parse_socket_frames(body_text: &str, line: usize, feed: &mut RawFeed) -> Result<(), FeedError> {
    for frame_text in body_text.split('\n') {
        let frame_text = frame_text.trim();
        if frame_text.is_empty() {
            continue;
        }
        let frame: Value = serde_json::from_str(frame_text).map_err(|e| FeedError::MalformedSocketFrame {
            line,
            detail: e.to_string(),
        })?;
        let obj = match frame.as_object() {
            Some(o) => o,
            None => {
                return Err(FeedError::UnrecognizedSocketFrame {
                    line,
                    excerpt: excerpt(frame_text),
                })
            }
        };
        if obj.is_empty() {
            feed.chrome.heartbeat_frames += 1;
            continue;
        }
        if obj.contains_key("id") {
            // Connect/subscribe/presence replies — transport handshake.
            feed.chrome.handshake_frames += 1;
            continue;
        }
        let push = match obj.get("push") {
            Some(p) => p,
            None => {
                return Err(FeedError::UnrecognizedSocketFrame {
                    line,
                    excerpt: excerpt(frame_text),
                })
            }
        };
        let channel = push.get("channel").and_then(|v| v.as_str()).unwrap_or("");
        if push.get("join").is_some() || push.get("leave").is_some() {
            feed.chrome.join_leave_frames += 1;
            continue;
        }
        let pub_data = match push.get("pub").and_then(|p| p.get("data")) {
            Some(d) => d,
            None => {
                return Err(FeedError::UnrecognizedSocketFrame {
                    line,
                    excerpt: excerpt(frame_text),
                })
            }
        };
        if !channel.starts_with("/table/t") {
            feed.chrome.other_channel_pubs += 1;
            feed.chrome.other_channels.insert(channel.to_string());
            if channel.starts_with("/player/p") {
                feed.chrome.player_channels.insert(channel.to_string());
            }
            continue;
        }
        let ptype = pub_data.get("packet_type").and_then(|v| v.as_str()).unwrap_or("<absent>");
        match ptype {
            "sequence" => feed.packets.push(parse_packet(pub_data, Transport::Socket, line)?),
            "single" => {
                feed.chrome.single_packets += 1;
                if let Some(arr) = pub_data.get("data").and_then(|d| d.as_array()) {
                    for n in arr {
                        if let Some(k) = n.get("type").and_then(|v| v.as_str()) {
                            *feed.chrome.single_kinds.entry(k.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
            other => {
                return Err(FeedError::UnknownPacketType {
                    line,
                    packet_type: other.to_string(),
                })
            }
        }
    }
    Ok(())
}

fn excerpt(s: &str) -> String {
    const MAX: usize = 120;
    if s.len() <= MAX {
        s.to_string()
    } else {
        let mut end = MAX;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

/// Parse a whole capture file (NDJSON text). Packets come back in ARRIVAL
/// order; nothing is deduplicated or collapsed yet — that is [`collapse`]'s
/// job, kept separate so the raw parse stays a pure envelope concern.
pub fn read_captures(text: &str) -> Result<RawFeed, FeedError> {
    let mut feed = RawFeed::default();
    for (idx, raw_line) in text.lines().enumerate() {
        let line = idx + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        feed.lines += 1;
        let v: Value = serde_json::from_str(trimmed).map_err(|e| FeedError::MalformedLine {
            line,
            detail: e.to_string(),
        })?;
        let received_at_ms = required_flex_u64(&v, "received_at_ms", line, "capture line")?;
        let kind = v
            .get("kind")
            .and_then(|k| k.as_str())
            .ok_or_else(|| FeedError::MissingField {
                line,
                context: "capture line".to_string(),
                field: "kind",
            })?;
        let payload = v.get("payload").ok_or_else(|| FeedError::MissingField {
            line,
            context: "capture line".to_string(),
            field: "payload",
        })?;
        match kind {
            "snapshot" => {
                let gamedatas = payload
                    .get("gamedatas")
                    .cloned()
                    .ok_or_else(|| FeedError::MissingField {
                        line,
                        context: "snapshot payload".to_string(),
                        field: "gamedatas",
                    })?;
                feed.snapshots.push(Snapshot {
                    received_at_ms,
                    captured_at_ms: payload.get("captured_at_ms").and_then(flex_u64),
                    href: payload.get("href").and_then(|h| h.as_str()).map(|s| s.to_string()),
                    gamedatas,
                });
            }
            "packets" => {
                let source = payload
                    .get("source")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| FeedError::MissingField {
                        line,
                        context: "packets payload".to_string(),
                        field: "source",
                    })?;
                let body_text = payload
                    .get("body_text")
                    .and_then(|b| b.as_str())
                    .ok_or_else(|| FeedError::MissingField {
                        line,
                        context: "packets payload".to_string(),
                        field: "body_text",
                    })?;
                match source {
                    "xhr" | "fetch" => parse_history_body(body_text, line, &mut feed)?,
                    "websocket" => parse_socket_frames(body_text, line, &mut feed)?,
                    other => {
                        return Err(FeedError::UnknownSource {
                            line,
                            source: other.to_string(),
                        })
                    }
                }
            }
            other => {
                return Err(FeedError::UnknownKind {
                    line,
                    kind: other.to_string(),
                })
            }
        }
    }
    Ok(feed)
}

fn is_moveless_chrome(pk: &Packet) -> bool {
    pk.notifications
        .iter()
        .all(|n| MOVELESS_CHROME_KINDS.contains(&n.kind.as_str()))
}

/// Socket-only filter for the merge check: the history form omits these kinds
/// entirely, so the comparable slice of a socket packet is what remains.
fn shared_kind_pairs(pk: &Packet) -> Vec<(String, String)> {
    pk.notifications
        .iter()
        .filter(|n| !SOCKET_ONLY_KINDS.contains(&n.kind.as_str()))
        .map(|n| (n.kind.clone(), n.uid.clone()))
        .collect()
}

fn all_kind_pairs(pk: &Packet) -> Vec<(String, String)> {
    pk.notifications
        .iter()
        .map(|n| (n.kind.clone(), n.uid.clone()))
        .collect()
}

/// Dedupe, collapse supersession, and merge the two feeds (plan §2 T2).
///
/// Order of operations, each step measured against the H0 corpus:
/// (a) select the table (`table` disambiguates a multi-table capture);
/// (b) HISTORY: key on `packet_id` across overlapping fetches — a repeated id
///     with identical content is a counted duplicate, differing content a STOP;
/// (c) SOCKET: dedupe on `packet_id` the same way, then walk arrival order
///     applying the STRUCTURAL supersession rule (`prevpacket != [pid-1]`
///     rewinds; marker required; a bare `move_id` regression is a STOP);
/// (d) drop moveless chrome packets, counted;
/// (e) MERGE on `move_id`: history is the spine where present, the socket form
///     attaches as enrichment, and the two forms of one move must agree on
///     their `(kind, uid)` sequence after socket-only filtering.
pub fn collapse(feed: &RawFeed, table: Option<u64>) -> Result<CollapsedFeed, FeedError> {
    // The table filter selects PACKETS only — a Snapshot carries no table id,
    // so every snapshot in the file rides through regardless (NOTE-5, H1
    // review). Harmless while the relay writes per-table capture files; if a
    // multi-table capture ever becomes real, snapshots need their own
    // attribution before this filter can honestly cover them.
    let tables: BTreeSet<u64> = feed.packets.iter().map(|p| p.table_id).collect();
    let table_id = match table {
        Some(t) => {
            if !tables.is_empty() && !tables.contains(&t) {
                return Err(FeedError::TableNotInCapture {
                    table: t,
                    present: tables.into_iter().collect(),
                });
            }
            t
        }
        None => match tables.len() {
            0 => return Err(FeedError::NoPackets),
            1 => *tables.iter().next().expect("len checked"),
            _ => {
                return Err(FeedError::MultipleTables {
                    tables: tables.into_iter().collect(),
                })
            }
        },
    };

    let mut report = FeedReport {
        snapshots: feed.snapshots.len(),
        chrome: feed.chrome.clone(),
        ..FeedReport::default()
    };

    // (b) History dedupe.
    let mut history: BTreeMap<u64, Packet> = BTreeMap::new();
    for pk in feed.packets.iter().filter(|p| p.table_id == table_id && p.transport == Transport::History) {
        report.history_packets_raw += 1;
        match history.get(&pk.packet_id) {
            Some(existing) if existing.same_content(pk) => {
                report.history_duplicates_dropped += 1;
            }
            Some(_) => {
                return Err(FeedError::DuplicatePacketDiffers {
                    transport: Transport::History,
                    packet_id: pk.packet_id,
                })
            }
            None => {
                history.insert(pk.packet_id, pk.clone());
            }
        }
    }

    // (c) Socket dedupe + structural supersession, in arrival order.
    let mut seen_socket: BTreeMap<u64, Packet> = BTreeMap::new();
    let mut accepted: Vec<Packet> = Vec::new();
    for pk in feed.packets.iter().filter(|p| p.table_id == table_id && p.transport == Transport::Socket) {
        report.socket_packets_raw += 1;
        match seen_socket.get(&pk.packet_id) {
            Some(existing) if existing.same_content(pk) => {
                report.socket_duplicates_dropped += 1;
                continue;
            }
            Some(_) => {
                return Err(FeedError::DuplicatePacketDiffers {
                    transport: Transport::Socket,
                    packet_id: pk.packet_id,
                })
            }
            None => {
                seen_socket.insert(pk.packet_id, pk.clone());
            }
        }
        let prev = pk.prevpacket.as_deref().unwrap_or(&[]);
        let is_undo = match prev {
            // The stream's GENESIS: the first packet of a from-table-open
            // capture has nothing before it and BGA sends an empty prevpacket
            // (measured live at the gate game, table 896153951 — the proving
            // corpus never exhibited this because its socket joined mid-game
            // at packet 100+). Empty ANYWHERE ELSE stays an unmeasured shape
            // and refuses below.
            [] if pk.packet_id == 1 => false,
            [p] if *p + 1 == pk.packet_id => false,
            [p] if *p + 1 < pk.packet_id => true,
            _ => {
                return Err(FeedError::UnexpectedPrevpacket {
                    packet_id: pk.packet_id,
                    prevpacket: prev.to_vec(),
                })
            }
        };
        if is_undo {
            if !pk.notifications.iter().any(|n| n.kind == "undoRestorePoint") {
                return Err(FeedError::StructuralBreakWithoutMarker {
                    packet_id: pk.packet_id,
                    prevpacket: prev.to_vec(),
                });
            }
            let boundary = prev[0];
            let superseded: Vec<u64> = accepted
                .iter()
                .filter(|a| a.packet_id > boundary && a.packet_id < pk.packet_id)
                .map(|a| a.packet_id)
                .collect();
            accepted.retain(|a| !(a.packet_id > boundary && a.packet_id < pk.packet_id));
            report.superseded.push(SupersessionEvent {
                undo_packet_id: pk.packet_id,
                undo_move_id: pk.move_id,
                superseded_packet_ids: superseded,
            });
        } else if let Some(m) = pk.move_id {
            // Belt for the shape the corpus never showed: a move_id rewind
            // with an innocent-looking prevpacket chain.
            let max_before = accepted.iter().filter_map(|a| a.move_id).max();
            if let Some(max) = max_before {
                if m < max {
                    return Err(FeedError::RegressionWithoutStructuralBreak {
                        packet_id: pk.packet_id,
                        move_id: m,
                        max_seen: max,
                    });
                }
            }
        }
        accepted.push(pk.clone());
    }

    // (d) Moveless chrome drops, both transports.
    let mut history_moves: BTreeMap<u64, Packet> = BTreeMap::new();
    for pk in history.into_values() {
        match pk.move_id {
            None => {
                if !is_moveless_chrome(&pk) {
                    return Err(FeedError::MovelessGamePacket {
                        transport: Transport::History,
                        packet_id: pk.packet_id,
                        kinds: pk.notifications.iter().map(|n| n.kind.clone()).collect(),
                    });
                }
                report.moveless_dropped_history += 1;
            }
            Some(m) => {
                if history_moves.insert(m, pk).is_some() {
                    return Err(FeedError::DuplicateMoveId {
                        transport: Transport::History,
                        move_id: m,
                    });
                }
            }
        }
    }
    let mut socket_moves: BTreeMap<u64, Packet> = BTreeMap::new();
    for pk in accepted {
        match pk.move_id {
            None => {
                if !is_moveless_chrome(&pk) {
                    return Err(FeedError::MovelessGamePacket {
                        transport: Transport::Socket,
                        packet_id: pk.packet_id,
                        kinds: pk.notifications.iter().map(|n| n.kind.clone()).collect(),
                    });
                }
                report.moveless_dropped_socket += 1;
            }
            Some(m) => {
                if socket_moves.insert(m, pk).is_some() {
                    return Err(FeedError::DuplicateMoveId {
                        transport: Transport::Socket,
                        move_id: m,
                    });
                }
            }
        }
    }

    // (e) Merge.
    let move_ids: BTreeSet<u64> = history_moves.keys().chain(socket_moves.keys()).copied().collect();
    let mut moves = Vec::with_capacity(move_ids.len());
    for m in &move_ids {
        let h = history_moves.get(m).cloned();
        let s = socket_moves.get(m).cloned();
        if let (Some(h_pk), Some(s_pk)) = (&h, &s) {
            let h_pairs = all_kind_pairs(h_pk);
            let s_pairs = shared_kind_pairs(s_pk);
            if h_pairs != s_pairs {
                return Err(FeedError::FeedDisagreement {
                    move_id: *m,
                    history: h_pairs,
                    socket_filtered: s_pairs,
                });
            }
            report.enriched += 1;
        }
        moves.push(CollapsedMove {
            move_id: *m,
            history: h,
            socket: s,
        });
    }
    report.moves = moves.len();
    if let Some(max) = move_ids.iter().max() {
        report.missing_move_ids = (1..=*max).filter(|m| !move_ids.contains(m)).collect();
    }

    Ok(CollapsedFeed {
        table_id,
        moves,
        snapshots: feed.snapshots.clone(),
        report,
    })
}

/// Convenience: parse + collapse in one call.
pub fn read_and_collapse(text: &str, table: Option<u64>) -> Result<CollapsedFeed, FeedError> {
    let feed = read_captures(text)?;
    collapse(&feed, table)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- hand-built line helpers -------------------------------------------

    fn line(kind: &str, payload: Value) -> String {
        serde_json::json!({"received_at_ms": 1, "kind": kind, "payload": payload}).to_string()
    }

    fn history_line(packets: Value) -> String {
        line(
            "packets",
            serde_json::json!({
                "source": "xhr",
                "body_text": serde_json::json!({
                    "status": 1,
                    "data": {"valid": 1, "data": packets}
                }).to_string(),
            }),
        )
    }

    fn history_packet(pid: u64, mid: Value, data: Value) -> Value {
        serde_json::json!({
            "packet_type": "history", "table_id": 42, "packet_id": pid.to_string(),
            "move_id": mid, "time": "100", "channel": "/table/t42", "data": data
        })
    }

    fn socket_line_for_table(table: u64, mut pk: Value) -> String {
        pk["table_id"] = serde_json::json!(table);
        line(
            "packets",
            serde_json::json!({
                "source": "websocket",
                "body_text": serde_json::json!({
                    "push": {"channel": format!("/table/t{table}"), "pub": {"data": pk}}
                }).to_string(),
            }),
        )
    }

    fn socket_line(pk: Value) -> String {
        socket_line_for_table(42, pk)
    }

    fn socket_packet(pid: u64, mid: Option<u64>, prev: u64, data: Value) -> Value {
        let mut pk = serde_json::json!({
            "packet_type": "sequence", "table_id": 42, "packet_id": pid,
            "time": 100, "prevpacket": [prev.to_string()], "id": 0, "data": data
        });
        if let Some(m) = mid {
            pk["move_id"] = serde_json::json!(m);
        }
        pk
    }

    fn notif(uid: &str, kind: &str) -> Value {
        serde_json::json!({"uid": uid, "type": kind, "log": "", "args": {}})
    }

    /// The GENESIS shape measured live at the gate game (table 896153951): a
    /// from-table-open capture's FIRST socket packet has an empty `prevpacket`
    /// — nothing precedes it. The proving corpus never showed this (its socket
    /// joined mid-game), so the collapse refused it as unmeasured and the
    /// follower held at packet 1 of a brand-new game.
    #[test]
    fn the_first_socket_packet_may_have_no_predecessor() {
        let mut pk = socket_packet(1, Some(1), 0, serde_json::json!([notif("u1", "assistantPlacement")]));
        pk["prevpacket"] = serde_json::json!([]);
        let out = read_and_collapse(&socket_line(pk), Some(42)).expect("an empty prevpacket at packet 1 is the stream's genesis, not an undo");
        assert_eq!(out.report.moves, 1, "the genesis packet's move must survive the collapse");
    }

    /// The second live-measured genesis-class shape (gate game, move 1): the
    /// SIMULTANEOUS opening placement rides a `gameStateMultipleActiveUpdate`
    /// framework notification that exists on the socket only — the merge check
    /// must treat it as socket-only chrome, not as feed disagreement.
    #[test]
    fn the_multi_active_opening_update_is_socket_only_chrome() {
        let mut p1 = socket_packet(1, Some(1), 0, serde_json::json!([
            notif("u1", "assistantPlacement"),
            notif("u2", "gameStateMultipleActiveUpdate"),
        ]));
        p1["prevpacket"] = serde_json::json!([]);
        let hist = history_line(serde_json::json!([history_packet(1, "1".into(), serde_json::json!([notif("u1", "assistantPlacement")]))]));
        let input = format!("{}\n{}", socket_line(p1).trim_end(), hist.trim_end());
        let out = read_and_collapse(&input, Some(42))
            .expect("the multi-active update filters out of the merge check as socket-only chrome");
        assert_eq!(out.report.enriched, 1, "the move must merge, not disagree");
    }

    /// The narrowness that keeps the genesis arm honest: an empty prevpacket
    /// ANYWHERE ELSE is still the unmeasured shape it always was — accepting it
    /// broadly could mask a rewind whose marker we never saw.
    #[test]
    fn an_empty_prevpacket_after_the_first_packet_still_refuses() {
        let p1 = socket_packet(1, Some(1), 0, serde_json::json!([notif("u1", "assistantPlacement")]));
        let mut p1 = p1; p1["prevpacket"] = serde_json::json!([]);
        let mut p2 = socket_packet(2, Some(2), 1, serde_json::json!([notif("u2", "assistantPlacement")]));
        p2["prevpacket"] = serde_json::json!([]);
        let input = format!("{}\n{}", socket_line(p1).trim_end(), socket_line(p2).trim_end());
        let err = read_and_collapse(&input, Some(42)).expect_err("packet 2 with an empty prevpacket is not a genesis");
        assert!(
            matches!(err, FeedError::UnexpectedPrevpacket { packet_id: 2, .. }),
            "the refusal must name packet 2's unmeasured shape: {err}"
        );
    }

    // ---- reader arms -------------------------------------------------------

    #[test]
    fn an_unknown_capture_kind_is_a_typed_error_not_a_skip() {
        let err = read_captures(&line("mystery", serde_json::json!({}))).unwrap_err();
        assert_eq!(
            err,
            FeedError::UnknownKind { line: 1, kind: "mystery".to_string() }
        );
    }

    #[test]
    fn an_unknown_payload_source_is_a_typed_error() {
        let err = read_captures(&line(
            "packets",
            serde_json::json!({"source": "carrier-pigeon", "body_text": "{}"}),
        ))
        .unwrap_err();
        assert_eq!(
            err,
            FeedError::UnknownSource { line: 1, source: "carrier-pigeon".to_string() }
        );
    }

    #[test]
    fn a_history_body_off_the_measured_shape_is_a_typed_error() {
        let bad = line(
            "packets",
            serde_json::json!({"source": "xhr", "body_text": "{\"data\": []}"}),
        );
        match read_captures(&bad).unwrap_err() {
            FeedError::MalformedHistoryBody { line: 1, .. } => {}
            other => panic!("expected MalformedHistoryBody, got {other:?}"),
        }
    }

    #[test]
    fn an_unknown_packet_type_is_a_typed_error() {
        let text = history_line(serde_json::json!([
            {"packet_type": "broadcast", "table_id": 42, "packet_id": "1", "move_id": "1", "data": []}
        ]));
        assert_eq!(
            read_captures(&text).unwrap_err(),
            FeedError::UnknownPacketType { line: 1, packet_type: "broadcast".to_string() }
        );
    }

    #[test]
    fn an_unrecognized_socket_frame_is_a_typed_error() {
        let text = line(
            "packets",
            serde_json::json!({"source": "websocket", "body_text": "{\"surprise\": true}"}),
        );
        match read_captures(&text).unwrap_err() {
            FeedError::UnrecognizedSocketFrame { line: 1, .. } => {}
            other => panic!("expected UnrecognizedSocketFrame, got {other:?}"),
        }
    }

    #[test]
    fn scalar_normalization_reads_strings_in_history_and_ints_on_the_socket() {
        // The §2 scalar-type trap: same packet id, spelled "7" and 7.
        let h = history_line(serde_json::json!([history_packet(7, "7".into(), serde_json::json!([]))]));
        let s = socket_line(socket_packet(7, Some(7), 6, serde_json::json!([])));
        let feed = read_captures(&format!("{h}\n{s}")).unwrap();
        assert_eq!(feed.packets.len(), 2);
        assert!(feed.packets.iter().all(|p| p.packet_id == 7 && p.move_id == Some(7)));
    }

    #[test]
    fn heartbeats_handshakes_and_foreign_channels_are_counted_chrome() {
        let frames = [
            "{}",
            "{\"id\": 1, \"subscribe\": {}}",
            "{\"push\": {\"channel\": \"/chat/general\", \"pub\": {\"data\": {\"x\": 1}}}}",
            "{\"push\": {\"channel\": \"/player/p11111111\", \"pub\": {\"data\": {\"x\": 1}}}}",
            "{\"push\": {\"channel\": \"/table/t42\", \"join\": {}}}",
        ]
        .join("\n");
        let text = line("packets", serde_json::json!({"source": "websocket", "body_text": frames}));
        let feed = read_captures(&text).unwrap();
        assert_eq!(feed.packets.len(), 0);
        assert_eq!(feed.chrome.heartbeat_frames, 1);
        assert_eq!(feed.chrome.handshake_frames, 1);
        assert_eq!(feed.chrome.other_channel_pubs, 2);
        assert_eq!(feed.chrome.join_leave_frames, 1);
        assert!(feed.chrome.player_channels.contains("/player/p11111111"));
    }

    // ---- collapse ----------------------------------------------------------

    #[test]
    fn duplicate_deliveries_with_identical_content_are_counted_and_dropped() {
        let s = socket_line(socket_packet(10, Some(5), 9, serde_json::json!([notif("u1", "assistantPlacement")])));
        let text = format!("{s}\n{s}");
        let collapsed = read_and_collapse(&text, None).unwrap();
        assert_eq!(collapsed.report.socket_duplicates_dropped, 1);
        assert_eq!(collapsed.moves.len(), 1);
    }

    #[test]
    fn a_duplicate_packet_id_with_different_content_is_a_stop() {
        let a = socket_line(socket_packet(10, Some(5), 9, serde_json::json!([notif("u1", "assistantPlacement")])));
        let b = socket_line(socket_packet(10, Some(5), 9, serde_json::json!([notif("u2", "assistantPlacement")])));
        let err = read_and_collapse(&format!("{a}\n{b}"), None).unwrap_err();
        assert_eq!(
            err,
            FeedError::DuplicatePacketDiffers { transport: Transport::Socket, packet_id: 10 }
        );
    }

    #[test]
    fn a_structural_rewind_supersedes_the_packets_it_skips() {
        // The measured shape: normal 10, 11, 12, then an undo packet 13 with
        // prevpacket [10] — 11 and 12 are superseded, whatever their move ids.
        let lines = [
            socket_line(socket_packet(10, Some(5), 9, serde_json::json!([notif("a", "assistantPlacement")]))),
            socket_line(socket_packet(11, Some(6), 10, serde_json::json!([notif("b", "assistantPlacement")]))),
            socket_line(socket_packet(12, Some(7), 11, serde_json::json!([notif("c", "presidentPlacement")]))),
            socket_line(socket_packet(13, Some(6), 10, serde_json::json!([notif("d", "undoRestorePoint")]))),
            socket_line(socket_packet(14, Some(7), 13, serde_json::json!([notif("e", "assistantPlacement")]))),
        ]
        .join("\n");
        let collapsed = read_and_collapse(&lines, None).unwrap();
        assert_eq!(collapsed.report.superseded.len(), 1);
        let ev = &collapsed.report.superseded[0];
        assert_eq!(ev.undo_packet_id, 13);
        assert_eq!(ev.superseded_packet_ids, vec![11, 12]);
        // Moves 5 (kept), 6 (now the undo marker), 7 (the replay).
        let mids: Vec<u64> = collapsed.moves.iter().map(|m| m.move_id).collect();
        assert_eq!(mids, vec![5, 6, 7]);
        let m7 = collapsed.moves.iter().find(|m| m.move_id == 7).unwrap();
        assert_eq!(m7.socket.as_ref().unwrap().notifications[0].uid, "e");
    }

    #[test]
    fn a_single_packet_undo_regresses_nothing_and_is_still_caught_structurally() {
        // Inventory §4.1: undos 149/170 superseded exactly ONE packet, so the
        // undo's move_id EQUALS the previous arrival's — a `<` test misses it.
        let lines = [
            socket_line(socket_packet(10, Some(5), 9, serde_json::json!([notif("a", "assistantPlacement")]))),
            socket_line(socket_packet(11, Some(6), 10, serde_json::json!([notif("b", "presidentPlacement")]))),
            socket_line(socket_packet(12, Some(6), 10, serde_json::json!([notif("c", "undoRestorePoint")]))),
        ]
        .join("\n");
        let collapsed = read_and_collapse(&lines, None).unwrap();
        assert_eq!(collapsed.report.superseded.len(), 1);
        assert_eq!(collapsed.report.superseded[0].superseded_packet_ids, vec![11]);
    }

    #[test]
    fn a_structural_rewind_without_the_marker_is_a_stop() {
        let lines = [
            socket_line(socket_packet(10, Some(5), 9, serde_json::json!([notif("a", "assistantPlacement")]))),
            socket_line(socket_packet(11, Some(6), 10, serde_json::json!([notif("b", "assistantPlacement")]))),
            socket_line(socket_packet(13, Some(6), 10, serde_json::json!([notif("c", "assistantPlacement")]))),
        ]
        .join("\n");
        let err = read_and_collapse(&lines, None).unwrap_err();
        assert_eq!(
            err,
            FeedError::StructuralBreakWithoutMarker { packet_id: 13, prevpacket: vec![10] }
        );
    }

    #[test]
    fn a_move_id_regression_without_a_structural_break_is_a_stop() {
        let lines = [
            socket_line(socket_packet(10, Some(5), 9, serde_json::json!([notif("a", "assistantPlacement")]))),
            socket_line(socket_packet(11, Some(4), 10, serde_json::json!([notif("b", "assistantPlacement")]))),
        ]
        .join("\n");
        let err = read_and_collapse(&lines, None).unwrap_err();
        assert_eq!(
            err,
            FeedError::RegressionWithoutStructuralBreak { packet_id: 11, move_id: 4, max_seen: 5 }
        );
    }

    #[test]
    fn a_moveless_packet_of_pure_chrome_is_dropped_and_counted() {
        let h = history_line(serde_json::json!([history_packet(30, Value::Null, serde_json::json!([]))]));
        let s = socket_line(socket_packet(30, None, 29, serde_json::json!([notif("w", "wakeupPlayers")])));
        let anchor = socket_line(socket_packet(31, Some(9), 30, serde_json::json!([notif("x", "assistantPlacement")])));
        let collapsed = read_and_collapse(&format!("{h}\n{s}\n{anchor}"), None).unwrap();
        assert_eq!(collapsed.report.moveless_dropped_history, 1);
        assert_eq!(collapsed.report.moveless_dropped_socket, 1);
        assert_eq!(collapsed.moves.len(), 1);
    }

    #[test]
    fn a_moveless_packet_carrying_a_game_notification_is_a_stop() {
        let s = socket_line(socket_packet(30, None, 29, serde_json::json!([notif("g", "presidentPlacement")])));
        let err = read_and_collapse(&s, None).unwrap_err();
        assert_eq!(
            err,
            FeedError::MovelessGamePacket {
                transport: Transport::Socket,
                packet_id: 30,
                kinds: vec!["presidentPlacement".to_string()],
            }
        );
    }

    #[test]
    fn the_merge_accepts_a_socket_form_whose_extra_kinds_are_all_socket_only() {
        // History serves the thin, gsc-free form; the socket form wraps the
        // same (kind, uid) pairs in gameStateChange framing. Measured to be
        // the shape of all 88 doubly-covered corpus moves.
        let h = history_line(serde_json::json!([history_packet(
            50,
            "20".into(),
            serde_json::json!([notif("u1", "assistantPlacement")])
        )]));
        let s = socket_line(socket_packet(
            50,
            Some(20),
            49,
            serde_json::json!([
                notif("g1", "gameStateChange"),
                notif("u1", "assistantPlacement"),
                notif("g2", "gameStateChange"),
                notif("r1", "updateReflexionTime"),
            ]),
        ));
        let collapsed = read_and_collapse(&format!("{h}\n{s}"), None).unwrap();
        assert_eq!(collapsed.report.enriched, 1);
        assert_eq!(collapsed.moves.len(), 1);
    }

    #[test]
    fn a_real_disagreement_between_the_feeds_is_a_stop() {
        let h = history_line(serde_json::json!([history_packet(
            50,
            "20".into(),
            serde_json::json!([notif("u1", "assistantPlacement")])
        )]));
        let s = socket_line(socket_packet(
            50,
            Some(20),
            49,
            serde_json::json!([notif("u2", "presidentPlacement")]),
        ));
        match read_and_collapse(&format!("{h}\n{s}"), None).unwrap_err() {
            FeedError::FeedDisagreement { move_id: 20, .. } => {}
            other => panic!("expected FeedDisagreement, got {other:?}"),
        }
    }

    #[test]
    fn a_multi_table_capture_demands_a_filter_and_honors_it() {
        let a = socket_line(socket_packet(10, Some(1), 9, serde_json::json!([notif("a", "assistantPlacement")])));
        let b = socket_line_for_table(43, socket_packet(10, Some(1), 9, serde_json::json!([notif("b", "assistantPlacement")])));
        let text = format!("{a}\n{b}");
        match read_and_collapse(&text, None).unwrap_err() {
            FeedError::MultipleTables { tables } => assert_eq!(tables, vec![42, 43]),
            other => panic!("expected MultipleTables, got {other:?}"),
        }
        let one = read_and_collapse(&text, Some(42)).unwrap();
        assert_eq!(one.table_id, 42);
        assert_eq!(one.moves.len(), 1);
        assert_eq!(
            read_and_collapse(&text, Some(99)).unwrap_err(),
            FeedError::TableNotInCapture { table: 99, present: vec![42, 43] }
        );
    }

    #[test]
    fn missing_move_ids_report_the_gap_honestly() {
        let a = history_line(serde_json::json!([history_packet(1, "1".into(), serde_json::json!([]))]));
        let b = history_line(serde_json::json!([history_packet(5, "4".into(), serde_json::json!([]))]));
        let collapsed = read_and_collapse(&format!("{a}\n{b}"), None).unwrap();
        assert_eq!(collapsed.report.missing_move_ids, vec![2, 3]);
    }
}
