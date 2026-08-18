//! The driven-fixture harness's RUNNER: argv in, one rendered report out.
//!
//! **Why this is a LIB module and not the example's body** (design §2). The
//! example `crates/yokohama-core/examples/driven.rs` is thin argv glue — parse,
//! call, print, exit — and everything it would otherwise contain lives here,
//! because an example's `#[cfg(test)]` module runs under NO command this repo
//! documents unless that command is written down at four doc sites (the
//! `eval_probe` precedent, and the standing rule that a suite no gate line names
//! is a suite that never runs). Putting the argv grammar and the rendering in
//! the lib means they are covered by `cargo test -p yokohama-core`, which every
//! per-edit gate already runs, and the harness ships no new test command at all.
//!
//! **What it does NOT do.** It computes nothing. Every number it prints comes
//! off [`driven`]'s own surface — [`driven::root_table`],
//! [`driven::drive_rooted`], [`driven::adjudicate_root`] — and this module only
//! decides which of them to call and how to lay the answer out. A runner that
//! re-derived a value would be a second derivation of a decision, which is the
//! thing `suggest`'s rule (spec §0.3) and this harness's MUST-1 resolution both
//! forbid.
//!
//! **Unknown is an ERROR at every level** — unknown subcommand, unknown flag,
//! unknown seat name, unknown contestant knob, a flag without its value, a
//! positional too many. `eval_probe`'s discipline, for its reason: a mistyped
//! flag that silently left a default in place puts a published figure under
//! knobs the run did not use.
//!
//! **The output is a `String`, not a stream of `println!`s.** That is what makes
//! rendering testable at all: a report is a value a cell can compare, hash, or
//! re-render for determinism, and the example's only job is to print it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::game::arena::Contestant;
use crate::game::driven::{
    self, Adjudication, DrivenOpts, DrivenOutcome, RegistryEntry, ReplyPolicy, RootDescriptor,
    RootSource, RootTable, SeatSpec,
};
use crate::game::lookahead::SearchStats;
use crate::game::rules::Rules;
use crate::game::seeds::WorldSeed;
use crate::types::{Move, PlayerId};

/// Every subcommand this runner answers. The parser reads it and
/// [`USAGE`]'s completeness pin asserts every member is documented.
pub const SUBCOMMANDS: &[&str] = &["list", "table", "run", "adjudicate"];

/// `list`'s flags: none.
pub const LIST_FLAGS: &[&str] = &[];

/// `table`'s flags.
pub const TABLE_FLAGS: &[&str] = &["contestant", "decision-seed"];

/// `run`'s flags. **`replications` is deliberately absent** — a single rooted
/// run has nothing to replicate, and accepting the flag would silently ignore
/// it.
pub const RUN_FLAGS: &[&str] = &["master", "seats"];

/// `adjudicate`'s flags.
pub const ADJUDICATE_FLAGS: &[&str] = &["master", "seats", "replications"];

/// The seat and contestant DEFAULT, spelled as the grammar spells it.
///
/// A string rather than a `Contestant` const on purpose: this is what a user
/// would type, `Contestant::from_str_name` fills every knob from its own shipped
/// defaults, and the runner therefore introduces no second opinion about what
/// "the shipped lookahead-v1" is.
pub const DEFAULT_SEAT: &str = "lookahead-v1";

/// `adjudicate`'s default K.
pub const DEFAULT_REPLICATIONS: u32 = 4;

/// How many ranked candidates `table` prints before it stops.
pub const RANKED_HEAD: usize = 10;

/// The name a `path:event` root's loader refusals are reported under. It has no
/// registry entry, so the refusal names the ARGUMENT FORM rather than an entry
/// nobody can look up.
pub const PATH_ROOT_NAME: &str = "path:event";

pub const USAGE: &str = "\
usage: driven <subcommand> [options]

  list
      Print the named-root REGISTRY: each entry's name, record path, event index,
      bot seat, recorded decision seed, whether the record file is PRESENT or
      MISSING right now, and what the root is for. An absent record is a MISS and
      never an error -- the session roots live in gitignored saves.

  table <root> [--contestant <spec>] [--decision-seed <u64>]
      Analyze ONE decision and apply nothing: the contestant echo, the sampled
      world (sampler seed, worlds, world digest), the winner, the ranking's shape
      (candidates / top value / tied-at-top / the <=1-point top band / distinct
      values), the ranked head, and ALL FIFTEEN SearchStats counters -- the
      mechanical separator's instrument, and the surface `why` cannot be (it
      prints 2 of the 15).

      --contestant <spec>    default `lookahead-v1`, the grammar's own shipped
                             default (beam 4, one world, 8 retreat samples,
                             UNCAPPED). The separator's wider beams are
                             `lookahead-v1:b=8` and `lookahead-v1:b=16`, and a
                             wider beam is EXPECTED to diverge from the recorded
                             move -- that divergence is the measurement.
                             ONLY `lookahead-v1` and its knob variants have a
                             root table at all. Every other arm is a TYPED
                             REFUSAL naming why, never an empty table: a rung
                             SAMPLES its move and values no candidate,
                             `greedy-v1` is a one-ply argmax that reports no
                             per-candidate vector and no counters, and `mcts-v1`
                             / `atomic-diag-v1` are labelled diagnostics no v1
                             consumer asks a root table about.
      --decision-seed <u64>  For a REGISTRY root this DEFAULTS to the entry's own
                             recorded seed, and the report then prints a
                             REPRODUCED / DIVERGED line against the move the
                             record carries at that event: the b=4 reproduction
                             check, as a command. For a <path>:<event> root it is
                             REQUIRED -- no seed is recorded anywhere for such a
                             root, and inventing one would answer a different
                             question from the one the record asks.

  run <root> --master <u64> [--seats <list>]
      Play the root to terminal ONCE and report it: provenance (root descriptor,
      resolved seat echoes, observation labels, per-seat streams), the result and
      standings, decisions and turns, the per-seat counters, and the outcome
      digest. Nothing is recorded to disk and no GameRecord is emitted -- a rooted
      run is not a from-setup game.

  adjudicate <root> --master <u64> [--replications <K>] [--seats <list>]
      Force EVERY root candidate in turn, play the remainder out K times under the
      named seats, and report each candidate's margins, mean/min/max and wins,
      plus whether every candidate reads alike. THE FIRST LINE OF THE OUTPUT is
      the robustness-read label and it is not optional: this is a playout under a
      named policy and never exact ground truth about a candidate.

      --replications <K>     default 4. Replication r plays in
                             mix(SALT_DRIVEN_REPLICATION, [master, r]), with each
                             seat's stream taken off that world.
      COST: this is `candidates x K` whole games. The candidate count is the
      ENGINE's at that root -- 1 at a forced decision, a handful at a Building
      point, up to the hundreds at a money-saturated Movement point -- so the
      sizing choices are the ROOT and the reply seats, never a flag here. Rungs
      reply in milliseconds; two uncapped lookahead seats on a wide root is hours.

roots (<root> above):
  <fixture-name>   a name from `list`. Its record resolves against the WORKSPACE
                   ROOT, so one name means one file wherever the binary is
                   launched from.
  <path>:<event>   an explicit record path and the event index the mover decided
                   in -- state_at's `k`, and the number `why <n>` prints. Split at
                   the LAST colon; the index is decimal; a relative path resolves
                   against the working directory. It is read through the SAME
                   loader a registry entry takes, so a missing / unreadable /
                   non-record / STALE file refuses identically. Only the entry's
                   own transcription checks (recorded move, recorded seat) are
                   entry-only, which is why `table` prints NOT CHECKED in place of
                   a REPRODUCED / DIVERGED line for a path root.

seats (--seats <a,b[,c,d]>, one spec per seat in seat order, default
`lookahead-v1` on EVERY seat):
  the contestant grammar   Contestant::from_str_name's, where unknown is an error
                           at every level and an unknown knob is never silently
                           ignored:
      greedy-v1[:m=<n>]
      lookahead-v1[:b=<n>,k=<n>,m=<n>,cap=<n>]
      uniform | retreat-averse | family-weighted[:p=<ppm>,k=<n>]
      atomic-diag-v1[:d=<n>,k=<n>,m=<n>] | mcts-v1[:cap=,c=,scale=,m=,leaf=]
  the declared REPLY menu  the three bare names below. They resolve to the same
                           contestants the grammar spells and differ only in what
                           the provenance records they were ASKED FOR, which is
                           the half that stops meaning what it means at D1's
                           identity freeze:
      family-weighted | greedy-at-final | lookahead-at-final
  `family-weighted` ALONE is the reply-menu slot; `family-weighted:p=...` is the
  knobbed rung through the contestant grammar. Both resolve to a rung.

masters:
  --master has NO DEFAULT on `run` and `adjudicate`. A master is drawn from OS
  entropy and RECORDED BY THE CALLER BEFORE THE RUN (the standing policy), so an
  instrument that drew one here would put the draw after the decision to run, in
  a process whose output is the thing the draw is supposed to be independent of.
  Decimal, or hex with a 0x prefix; `_` separators are ignored.

Unknown subcommands, unknown flags, unknown seat names, unknown contestant knobs
and stray positionals are all ERRORS carrying this text -- never silent no-ops.
A REPEATED flag is an ERROR too, which is a deliberate departure from
eval_probe's last-wins: dropping the first value silently is the same
substitution failure one level down from a mistyped flag.
";

// ---------------------------------------------------------------------------
// The parsed command
// ---------------------------------------------------------------------------

/// WHICH root a subcommand was pointed at, before anything is loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootRef {
    /// A [`driven::registry`] entry, by name.
    Fixture(String),
    /// An explicit record file and the event index the mover decided in.
    Path { path: PathBuf, event_index: usize },
}

/// One parsed command line. Pure data — nothing here has read a file or played a
/// move, which is what lets the whole grammar be tested without a corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    List,
    Table {
        root: RootRef,
        contestant: Contestant,
        /// `None` means "take the registry entry's recorded seed", which is only
        /// available for a [`RootRef::Fixture`] — a path root with no seed is
        /// refused at execution with the sentence that says why.
        decision_seed: Option<u64>,
    },
    Run {
        root: RootRef,
        master: u64,
        /// `None` means "the default seat on every seat", which needs the root's
        /// player count and so cannot be resolved at parse time.
        seats: Option<Vec<SeatSpec>>,
    },
    Adjudicate {
        root: RootRef,
        master: u64,
        replications: u32,
        seats: Option<Vec<SeatSpec>>,
    },
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a command line (argv WITHOUT the program name).
pub fn parse(args: &[String]) -> Result<Command, String> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(format!("a subcommand is required\n\n{USAGE}"));
    };
    let rest = &args[1..];
    match subcommand {
        "list" => {
            let (positional, _) = split_args(rest, LIST_FLAGS)?;
            if !positional.is_empty() {
                return Err(format!(
                    "list takes no positional argument, got {positional:?}\n\n{USAGE}"
                ));
            }
            Ok(Command::List)
        }
        "table" => {
            let (positional, flags) = split_args(rest, TABLE_FLAGS)?;
            let root = one_root(&positional, "table")?;
            let contestant = match flags.get("contestant") {
                Some(spec) => Contestant::from_str_name(spec)
                    .map_err(|e| format!("--contestant {spec:?}: {e}\n\n{USAGE}"))?,
                None => default_contestant()?,
            };
            let decision_seed = flags.get("decision-seed").map(|raw| parse_u64(raw)).transpose()?;
            Ok(Command::Table { root, contestant, decision_seed })
        }
        "run" => {
            let (positional, flags) = split_args(rest, RUN_FLAGS)?;
            let root = one_root(&positional, "run")?;
            Ok(Command::Run {
                root,
                master: master_of(&flags, "run")?,
                seats: seats_of(&flags)?,
            })
        }
        "adjudicate" => {
            let (positional, flags) = split_args(rest, ADJUDICATE_FLAGS)?;
            let root = one_root(&positional, "adjudicate")?;
            let replications = match flags.get("replications") {
                Some(raw) => u32::try_from(parse_u64(raw)?)
                    .map_err(|_| format!("--replications {raw:?} is not a u32\n\n{USAGE}"))?,
                None => DEFAULT_REPLICATIONS,
            };
            Ok(Command::Adjudicate {
                root,
                master: master_of(&flags, "adjudicate")?,
                replications,
                seats: seats_of(&flags)?,
            })
        }
        other => Err(format!("unknown subcommand {other:?}\n\n{USAGE}")),
    }
}

/// argv split into positionals and `--flag value` pairs.
///
/// EVERY flag this runner has takes a value, so there is no bare-flag arm — and
/// a valueless flag is therefore an error rather than a silent `true`. An
/// unknown flag is an error too (`eval_probe`'s rule): a mistyped
/// `--decision-see` that quietly left the entry's recorded seed in place would
/// print a reproduction verdict about a decision nobody asked for. A REPEATED
/// flag is an error as well rather than last-wins, since losing the first value
/// is the same silent-substitution failure one level down.
fn split_args(
    args: &[String],
    valued: &[&str],
) -> Result<(Vec<String>, BTreeMap<String, String>), String> {
    let mut positional = Vec::new();
    let mut flags: BTreeMap<String, String> = BTreeMap::new();
    let mut i = 0;
    while i < args.len() {
        if let Some(name) = args[i].strip_prefix("--") {
            if !valued.contains(&name) {
                return Err(format!("unknown flag --{name}\n\n{USAGE}"));
            }
            let value =
                args.get(i + 1).ok_or_else(|| format!("--{name} needs a value\n\n{USAGE}"))?;
            if flags.insert(name.to_string(), value.clone()).is_some() {
                return Err(format!("--{name} was given twice\n\n{USAGE}"));
            }
            i += 2;
            continue;
        }
        positional.push(args[i].clone());
        i += 1;
    }
    Ok((positional, flags))
}

/// The ONE root every subcommand but `list` takes.
fn one_root(positional: &[String], subcommand: &str) -> Result<RootRef, String> {
    match positional {
        [only] => parse_root_ref(only),
        other => Err(format!(
            "{subcommand} takes exactly ONE root, got {}: {other:?}\n\n{USAGE}",
            other.len()
        )),
    }
}

/// `--master`, which has no default anywhere.
fn master_of(flags: &BTreeMap<String, String>, subcommand: &str) -> Result<u64, String> {
    let raw = flags.get("master").ok_or_else(|| {
        format!(
            "{subcommand} requires --master, which has no default: a master is drawn from OS \
             entropy and RECORDED BY THE CALLER BEFORE THE RUN (the standing policy), so this \
             instrument takes the recorded value rather than drawing one — a draw made here would \
             sit after the decision to run, in a process whose output is the thing the draw is \
             supposed to be independent of\n\n{USAGE}"
        )
    })?;
    parse_u64(raw)
}

/// `--seats`, left `None` when absent because the default needs the root's
/// player count and nothing here has read the root yet.
fn seats_of(flags: &BTreeMap<String, String>) -> Result<Option<Vec<SeatSpec>>, String> {
    flags.get("seats").map(|raw| parse_seats(raw)).transpose()
}

/// [`DEFAULT_SEAT`] through the grammar. Propagated rather than unwrapped: a
/// runner that panicked on its own default would be worse than one that says so.
fn default_contestant() -> Result<Contestant, String> {
    Contestant::from_str_name(DEFAULT_SEAT)
        .map_err(|e| format!("the default contestant {DEFAULT_SEAT:?} does not parse: {e}"))
}

/// `<path>:<event>` or a bare registry name.
pub fn parse_root_ref(raw: &str) -> Result<RootRef, String> {
    let Some((path, event)) = raw.rsplit_once(':') else {
        if raw.is_empty() {
            return Err(format!("an empty root names nothing\n\n{USAGE}"));
        }
        return Ok(RootRef::Fixture(raw.to_string()));
    };
    // The LAST colon, so a path that contains one still reads. A path with a
    // colon and no event index is a user error rather than a fixture name,
    // because a fixture name with a colon in it does not exist.
    if path.is_empty() {
        return Err(format!("{raw:?} has no path before its last colon\n\n{USAGE}"));
    }
    let event_index = event.parse::<usize>().map_err(|e| {
        format!(
            "{raw:?} carries a colon, so it reads as <path>:<event> split at the LAST colon — but \
             {event:?} is not a decimal event index: {e}\n\n{USAGE}"
        )
    })?;
    Ok(RootRef::Path { path: PathBuf::from(path), event_index })
}

/// ONE seat spec: the reply menu's three bare names, else the contestant
/// grammar.
///
/// The menu is read FIRST and only on an EXACT bare name, which is what keeps
/// `family-weighted` (the menu slot) and `family-weighted:p=…` (the knobbed rung
/// through the contestant grammar) distinct. Both resolve to the same kind of
/// seat; what differs is the `requested` label the provenance records, and that
/// label is the half whose meaning moves at D1's identity freeze.
pub fn parse_seat(spec: &str) -> Result<SeatSpec, String> {
    for policy in
        [ReplyPolicy::FamilyWeighted, ReplyPolicy::GreedyAtFinal, ReplyPolicy::LookaheadAtFinal]
    {
        if spec == policy.as_str() {
            return Ok(SeatSpec::Reply(policy));
        }
    }
    Contestant::from_str_name(spec).map(SeatSpec::Contestant).map_err(|e| {
        format!("seat {spec:?} is neither a reply-menu name nor a contestant: {e}\n\n{USAGE}")
    })
}

/// A comma-separated seat list, one spec per seat in seat order.
pub fn parse_seats(raw: &str) -> Result<Vec<SeatSpec>, String> {
    if raw.trim().is_empty() {
        return Err(format!("--seats needs at least one seat\n\n{USAGE}"));
    }
    raw.split(',').map(|spec| parse_seat(spec.trim())).collect()
}

/// `eval_probe`'s spelling, so a master typed for one instrument means the same
/// thing typed for the other: decimal, or `0x` hex, with `_` separators ignored.
pub fn parse_u64(raw: &str) -> Result<u64, String> {
    let cleaned = raw.replace('_', "");
    let parsed = match cleaned.strip_prefix("0x").or_else(|| cleaned.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(hex, 16),
        None => cleaned.parse::<u64>(),
    };
    parsed.map_err(|e| format!("{raw:?} is not a u64: {e}"))
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// The fifteen [`SearchStats`] counters as `(name, value)`, in declaration
/// order.
///
/// **The body DESTRUCTURES `SearchStats` with no `..`**, which is the whole
/// point: a sixteenth counter added to that type is a COMPILE ERROR here, to be
/// answered with a row. A `stats.field` list would silently stay fifteen, and
/// `why` printing 2 of 15 is exactly the gap this table exists to close.
pub fn counter_rows(stats: &SearchStats) -> [(&'static str, u64); 15] {
    let SearchStats {
        root_candidates,
        nodes,
        handles,
        route_dfs_nodes,
        forced_nodes,
        leaves,
        scored,
        root_scored,
        budget_binds,
        budget_starved,
        placement_cap_binds,
        max_line,
        ply_cap_hits,
        free_extension_steps,
        free_cap_hits,
    } = *stats;
    [
        ("root_candidates", root_candidates),
        ("nodes", nodes),
        ("handles", handles),
        ("route_dfs_nodes", route_dfs_nodes),
        ("forced_nodes", forced_nodes),
        ("leaves", leaves),
        ("scored", scored),
        ("root_scored", root_scored),
        ("budget_binds", budget_binds),
        ("budget_starved", budget_starved),
        ("placement_cap_binds", placement_cap_binds),
        ("max_line", u64::from(max_line)),
        ("ply_cap_hits", ply_cap_hits),
        ("free_extension_steps", free_extension_steps),
        ("free_cap_hits", free_cap_hits),
    ]
}

/// `Some(n)` as `n`, `None` as `-` — the counter-absence distinction, rendered.
/// A `None` counter means "this contestant has no such quantity" and must never
/// print as a zero.
fn opt(value: Option<u64>) -> String {
    value.map_or_else(|| "-".to_string(), |v| v.to_string())
}

/// A RENDER-ONLY 16-hex fold of an already-computed reproduction string.
///
/// **Why this exists.** [`Adjudication::digest`] is `candidates × K` long — it
/// folds in every playout's world, margin, decision count and terminal digest —
/// so a real 139-candidate × 2-replication table digests to ~15 KB. Printed
/// raw, that is one "line" nobody can paste into a triage note, and a
/// reproduction check nobody performs is not a check. This is `state_digest`'s
/// own recipe (FNV-1a 64, 16 hex) over the digest STRING, so the report carries
/// a value a reader can compare at a glance.
///
/// **It is a DISPLAY shortening and NEVER an identity.** The digest function is
/// untouched and stays the authority: reproduction compares
/// `Adjudication::digest()` itself. Nothing writes this fold to disk, nothing
/// parses it back, and no record stamps it — which is exactly why it is
/// hand-rolled here rather than reaching for `game::semantics`'s (private) fold,
/// whose recipe is a *stored* identity this module must not start depending on.
fn digest_fold(digest: &str) -> String {
    // FNV-1a 64: the standard offset basis and prime, wrapping because a hash
    // is not a count.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in digest.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// The root descriptor as one line: origin kind, root digest, event index.
fn root_line(root: &RootDescriptor) -> String {
    format!(
        "{}@{}  event={}",
        root.origin.kind(),
        root.root_digest,
        root.event_index.map_or_else(|| "-".to_string(), |n| n.to_string()),
    )
}

/// What a `table` report knows about its root BEYOND the table itself.
#[derive(Debug, Clone, Copy)]
pub struct TableContext<'a> {
    /// What the caller typed — a fixture name or a `path:event` spec.
    pub asked_for: &'a str,
    /// The resolved root's identity.
    pub root: &'a RootDescriptor,
    /// The move the registry entry records at this event, when there is one.
    pub recorded_move: Option<&'a Move>,
    /// The seed the registry entry records for this decision, when there is one.
    pub recorded_seed: Option<u64>,
}

/// The registry, with each record's presence resolved against `anchor`.
///
/// **Absence is reported, never refused.** The session roots live in gitignored
/// saves, so a checkout without them must still be able to ask what the registry
/// names — and a runner that errored on the first miss would be unusable on
/// exactly the machine where the list matters most.
pub fn render_list(entries: &[RegistryEntry], anchor: &Path) -> String {
    let mut lines = vec![
        "=== driven registry ===".to_string(),
        format!("  {} entries, resolved against {}", entries.len(), anchor.display()),
        "  a MISSING record is a MISS and never a failure: the session roots live in gitignored \
         saves"
            .to_string(),
    ];
    for entry in entries {
        let presence = if anchor.join(entry.path).exists() { "PRESENT" } else { "MISSING" };
        lines.push(String::new());
        lines.push(format!("  {}  [{presence}]", entry.name));
        lines.push(format!("      path    {}", entry.path));
        lines.push(format!(
            "      event   {}   seat p{}   decision seed {:#018x}",
            entry.event_index, entry.bot_seat.0, entry.decision_seed,
        ));
        lines.push(format!(
            "      move    {}",
            entry
                .recorded_move
                .as_ref()
                .map_or_else(|| "(none recorded)".to_string(), |mv| format!("{mv:?}")),
        ));
        lines.push(format!("      note    {}", entry.note));
    }
    lines.join("\n")
}

/// ONE decision, analyzed — the mechanical separator's report.
pub fn render_root_table(table: &RootTable, ctx: &TableContext<'_>) -> String {
    let mut lines = vec![
        "=== driven table ===".to_string(),
        format!("  root           {}   {}", ctx.asked_for, root_line(ctx.root)),
        format!("  seat           p{}", table.seat.0),
        format!("  contestant     {}", table.echo),
        format!("  decision seed  {:#018x}", table.decision_seed),
        format!(
            "  world          sampler_seed={:#018x}  worlds={}  digest={}",
            table.world.sampler_seed, table.world.worlds, table.world.digest,
        ),
        format!("  winner         {:?}", table.winner),
        format!("  winner is a maximiser: {}", table.winner_is_a_maximiser()),
    ];
    lines.extend(reproduction_lines(table, ctx));
    lines.push(String::new());
    lines.push(format!(
        "  shape          candidates={}  top={}  tied-at-top={}  top-band(<={} pt)={}  \
         distinct={}",
        table.candidates(),
        table.top_value().map_or_else(|| "-".to_string(), |v| format!("{v:+}")),
        table.tied_at_top(),
        driven::TOP_BAND_POINTS,
        table.top_band(),
        table.distinct_values(),
    ));
    lines.push(String::new());
    lines.push(format!(
        "  ranked head ({} of {}, best first; margin is value - top):",
        table.ranked.len().min(RANKED_HEAD),
        table.ranked.len(),
    ));
    let top = table.top_value().unwrap_or(0);
    for (index, score) in table.ranked.iter().take(RANKED_HEAD).enumerate() {
        lines.push(format!(
            "    {:>3}.  {:>8}  {:>8}   {:?}",
            index + 1,
            format!("{:+}", score.value),
            format!("{:+}", score.value - top),
            score.mv,
        ));
    }
    lines.push(String::new());
    lines.push("  search counters (all fifteen):".to_string());
    for (name, value) in counter_rows(&table.stats) {
        lines.push(format!("    {name:<22} {value}"));
    }
    lines.join("\n")
}

/// The REPRODUCED / DIVERGED / NOT CHECKED verdict, and its override qualifier.
///
/// The check is the mechanical separator's step one, as a command: at the
/// entry's own recorded seed the shipped b=4 arm should pick the move the record
/// carries, and a wider beam is EXPECTED to diverge — that divergence is the
/// measurement rather than a fault. What the qualifier exists for is the case in
/// between: a seed the caller supplied is no longer the entry's own question, so
/// the verdict is still printed and is labelled as not being the entry's check.
fn reproduction_lines(table: &RootTable, ctx: &TableContext<'_>) -> Vec<String> {
    let event = ctx.root.event_index.map_or_else(|| "-".to_string(), |n| n.to_string());
    let Some(recorded) = ctx.recorded_move else {
        return vec![format!(
            "  reproduction   NOT CHECKED: this root carries no recorded move to compare the \
             winner against (only a registry entry does)",
        )];
    };
    let mut lines = vec![if *recorded == table.winner {
        format!("  REPRODUCED     the winner IS the move the record carries at event {event}")
    } else {
        format!(
            "  DIVERGED       the winner is {:?}; the record carries {recorded:?} at event {event}",
            table.winner,
        )
    }];
    if let Some(recorded_seed) = ctx.recorded_seed {
        if recorded_seed != table.decision_seed {
            lines.push(format!(
                "                 (the decision seed was OVERRIDDEN — {:#018x} against the \
                 entry's recorded {recorded_seed:#018x} — so this is not the entry's own \
                 reproduction check)",
                table.decision_seed,
            ));
        }
    }
    lines
}

/// One rooted run.
pub fn render_outcome(outcome: &DrivenOutcome, asked_for: &str) -> String {
    let provenance = &outcome.provenance;
    let mut lines = vec![
        "=== driven run ===".to_string(),
        format!("  root           {asked_for}   {}", root_line(&provenance.root)),
        format!(
            "  semantics      rev={}  rules={}",
            provenance.semantics_rev, provenance.rules_hash,
        ),
        format!("  master         {:#018x}", outcome.master.0),
        String::new(),
        "  seats (as asked for, as resolved, and as the driver's own bench witnessed them):"
            .to_string(),
    ];
    for seat in &provenance.seats {
        lines.push(format!(
            "    p{}  requested={}  echo={}  observation={}  stream={:#018x}",
            seat.seat, seat.requested, seat.echo, seat.observation, seat.agent_seed,
        ));
    }
    lines.push(String::new());
    lines.push(format!(
        "  result         winner=p{}  decisions={}  turns={}",
        outcome.result.winner.0, outcome.decisions, outcome.final_state.turn_number,
    ));
    let standings: Vec<String> = outcome
        .result
        .standings
        .iter()
        .map(|s| format!("#{} p{}={}", s.rank, s.player.0, s.breakdown.total))
        .collect();
    lines.push(format!("  standings      {}", standings.join("  ")));
    lines.push(format!(
        "  explanations   {}  (identity holds: {})",
        opt(outcome.explanations),
        outcome.explanation_identity_holds(),
    ));
    lines.push(String::new());
    lines.push("  per-seat counters (`-` means this contestant has no such quantity):".to_string());
    for seat in &outcome.seats {
        let c = &seat.counters;
        lines.push(format!(
            "    p{}  moves={}  explained={}  scored={}  root_scored={}  binds={}  starved={}  \
             ply_cap={}  free_cap={}",
            seat.seat.0,
            seat.applied_moves,
            opt(c.explained),
            opt(c.candidates_scored),
            opt(c.root_scored),
            opt(c.budget_binds),
            opt(c.budget_starved_decisions),
            opt(c.ply_cap_hits),
            opt(c.free_cap_hits),
        ));
    }
    lines.push(String::new());
    lines.push(format!("  digest         {}", outcome.digest()));
    lines.join("\n")
}

/// One adjudication. **Its FIRST LINE is [`driven::ROBUSTNESS_READ`] verbatim**
/// (design §3, review N-7): the label rides the OUTPUT and not only the docs, so
/// a table pasted into a triage note carries the claim it is entitled to make.
///
/// **Its LAST line is a FOLD of the reproduction digest, not the digest** — see
/// [`digest_fold`]. `Adjudication::digest()` stays the authority a reproduction
/// compares; what this report prints is a 16-hex hash of it, because the string
/// itself is `candidates × K` long and a report has to be quotable.
pub fn render_adjudication(adjudication: &Adjudication, asked_for: &str) -> String {
    let provenance = &adjudication.provenance;
    let mut lines = vec![
        driven::ROBUSTNESS_READ.to_string(),
        String::new(),
        "=== driven adjudicate ===".to_string(),
        format!("  root           {asked_for}   {}", root_line(&provenance.root)),
        format!(
            "  semantics      rev={}  rules={}",
            provenance.semantics_rev, provenance.rules_hash,
        ),
        format!("  mover          p{}   root turn={}", adjudication.mover.0, adjudication.root_turn),
        format!(
            "  master         {:#018x}   replications={}",
            adjudication.master.0, adjudication.replications,
        ),
        String::new(),
        "  seats:".to_string(),
    ];
    for seat in &provenance.seats {
        lines.push(format!(
            "    p{}  requested={}  echo={}",
            seat.seat, seat.requested, seat.echo,
        ));
    }
    lines.push(String::new());
    lines.push(format!(
        "  candidates     {}   all candidates equal: {}   (elementwise over the {} \
         replications; vacuously true below 2 candidates, which is a FORCED position or a \
         FINISHED game rather than a dead one)",
        adjudication.candidates(),
        adjudication.all_candidates_equal(),
        adjudication.replications,
    ));
    lines.push(String::new());
    lines.push(format!(
        "  {:>3}  {:>10}  {:>8}  {:>8}  {:>8}  {:>6}   candidate / margins",
        "#", "mean", "min", "max", "wins", "k",
    ));
    for (index, row) in adjudication.rows.iter().enumerate() {
        let margins: Vec<String> = row.margins().iter().map(|m| format!("{m:+}")).collect();
        lines.push(format!(
            "  {:>3}  {:>10.2}  {:>8}  {:>8}  {:>8}  {:>6}   {:?}",
            index + 1,
            row.mean_margin(),
            row.min_margin().map_or_else(|| "-".to_string(), |m| format!("{m:+}")),
            row.max_margin().map_or_else(|| "-".to_string(), |m| format!("{m:+}")),
            format!("{}/{}", row.wins(), row.playouts.len()),
            row.playouts.len(),
            row.candidate,
        ));
        lines.push(format!(
            "       margins [{}]  continuation={}",
            margins.join(", "),
            row.continuation_digest,
        ));
    }
    lines.push(String::new());
    // The witness is the FIRST PLAYOUT's provenance, so its root descriptor
    // names that playout's own starting position — root + the first candidate —
    // and NOT the adjudicated root. Said out loud because the two are one field
    // name apart and a reader who assumed otherwise would mis-read every stream.
    match &adjudication.witness {
        Some(witness) => lines.push(format!(
            "  witness        the FIRST PLAYOUT's stamped provenance: {} (root + candidate 1, \
             not the adjudicated root; world {:#018x})",
            root_line(&witness.root),
            witness.master,
        )),
        None => lines.push(
            "  witness        none: nothing was played (a terminal root adjudicates to an empty \
             table)"
                .to_string(),
        ),
    }
    // The FOLD, not the string: see `digest_fold` for why a raw
    // `Adjudication::digest()` is unquotable on a real table.
    lines.push(format!(
        "  digest         fnv1a64:{}   (of the adjudication-v1 reproduction string)",
        digest_fold(&adjudication.digest()),
    ));
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

/// Parse `args`, do what they say, and return the report to print.
pub fn run(args: &[String]) -> Result<String, String> {
    run_with(args, &driven::registry(), &driven::workspace_root())
}

/// [`run`] against an EXPLICIT registry and anchor.
///
/// The test affordance `RegistryEntry::load_from` documents, one level up: every
/// shipped entry points at a gitignored save, so a cell that could only ask the
/// real registry could only ever exercise the MISS. A cell supplies its own
/// entries and a temp-dir anchor and owns every file under it.
///
/// `pub(crate)` deliberately — a runner resolving registry paths against
/// anything but [`driven::workspace_root`] would name a different file for the
/// same entry name depending on where it was launched.
pub(crate) fn run_with(
    args: &[String],
    entries: &[RegistryEntry],
    anchor: &Path,
) -> Result<String, String> {
    let command = parse(args)?;
    let rules = Rules::standard();
    match command {
        Command::List => Ok(render_list(entries, anchor)),

        Command::Table { root, contestant, decision_seed } => {
            let asked = resolve(&root, entries, anchor, &rules)?;
            let seed = match decision_seed {
                Some(seed) => seed,
                None => asked.recorded_seed.ok_or_else(|| {
                    format!(
                        "{}: --decision-seed is REQUIRED for a <path>:<event> root. No decision \
                         seed is recorded anywhere for one — the CLI's own \
                         splitmix64(bot_seed ^ event_count) derivation lives in yokohama-cli and \
                         core does not re-implement it — so inventing a seed here would answer a \
                         different question from the one the record asks. A REGISTRY root takes \
                         its entry's own recorded seed by default.\n\n{USAGE}",
                        asked.asked_for,
                    )
                })?,
            };
            let (state, descriptor) = asked
                .source
                .resolve(&rules)
                .map_err(|e| format!("{}: no root: {e}", asked.asked_for))?;
            // The decision analyzed is the MOVER's. An entry that records
            // another seat as the bot there is pointed at a different decision
            // — and the loader cannot catch it, since the record does carry the
            // recorded move at that index whoever played it.
            if let Some(seat) = asked.bot_seat {
                if seat != state.current_player {
                    return Err(format!(
                        "{}: the entry records seat p{} at event {}, but p{} is on the move there \
                         — the entry points at another decision",
                        asked.asked_for, seat.0, asked.event_index, state.current_player.0,
                    ));
                }
            }
            let table = driven::root_table(&state, state.current_player, contestant, &rules, seed)
                .map_err(|e| format!("{}: no root table: {e}", asked.asked_for))?;
            Ok(render_root_table(
                &table,
                &TableContext {
                    asked_for: &asked.asked_for,
                    root: &descriptor,
                    recorded_move: asked.recorded_move.as_ref(),
                    recorded_seed: asked.recorded_seed,
                },
            ))
        }

        Command::Run { root, master, seats } => {
            let asked = resolve(&root, entries, anchor, &rules)?;
            let seats = seats_or_default(seats, &asked, &rules)?;
            let outcome = driven::drive_rooted(
                &asked.source,
                &seats,
                &rules,
                WorldSeed(master),
                DrivenOpts::default(),
            )
            .map_err(|e| format!("{}: the run did not complete: {e}", asked.asked_for))?;
            Ok(render_outcome(&outcome, &asked.asked_for))
        }

        Command::Adjudicate { root, master, replications, seats } => {
            let asked = resolve(&root, entries, anchor, &rules)?;
            let seats = seats_or_default(seats, &asked, &rules)?;
            let adjudication = driven::adjudicate_root(
                &asked.source,
                &seats,
                &rules,
                WorldSeed(master),
                replications,
                DrivenOpts::default(),
            )
            .map_err(|e| format!("{}: the adjudication did not complete: {e}", asked.asked_for))?;
            Ok(render_adjudication(&adjudication, &asked.asked_for))
        }
    }
}

/// A root REFERENCE turned into the material a subcommand needs: the source
/// itself, plus whatever the naming vocabulary happened to record about it.
///
/// The three `Option`s are all `Some` for a registry entry and all `None` for a
/// `<path>:<event>` root, and that asymmetry is the point rather than an
/// oversight — a path names a position and nothing else, so there is no recorded
/// seed to default to, no recorded move to reproduce against, and no recorded
/// seat to check the mover against.
struct Asked {
    /// What the caller typed, for report headers and error text.
    asked_for: String,
    source: RootSource,
    event_index: usize,
    recorded_seed: Option<u64>,
    recorded_move: Option<Move>,
    bot_seat: Option<PlayerId>,
}

fn resolve(
    root: &RootRef,
    entries: &[RegistryEntry],
    anchor: &Path,
    rules: &Rules,
) -> Result<Asked, String> {
    match root {
        RootRef::Fixture(name) => {
            let entry = entries.iter().find(|entry| entry.name == name).ok_or_else(|| {
                let known: Vec<&str> = entries.iter().map(|entry| entry.name).collect();
                format!(
                    "unknown root {name:?}: no registry entry by that name, and no ':' in it to \
                     read as <path>:<event>. Known entries: {}",
                    if known.is_empty() { "(none)".to_string() } else { known.join(", ") },
                )
            })?;
            let source = entry.load_from(anchor, rules).map_err(|e| e.to_string())?;
            Ok(Asked {
                asked_for: entry.name.to_string(),
                source,
                event_index: entry.event_index,
                recorded_seed: Some(entry.decision_seed),
                recorded_move: entry.recorded_move.clone(),
                bot_seat: Some(entry.bot_seat),
            })
        }
        RootRef::Path { path, event_index } => {
            // THE SAME loader a registry entry takes, so Missing / Read / Parse
            // / Stale fire identically however the root was named. Only the
            // entry's own transcription checks stay entry-side; a path has
            // nothing to transcribe against.
            let record = driven::load_record(PATH_ROOT_NAME, path, rules)
                .map_err(|e| e.to_string())?;
            Ok(Asked {
                asked_for: format!("{}:{event_index}", path.display()),
                source: RootSource::RecordPrefix { record, event_index: *event_index },
                event_index: *event_index,
                recorded_seed: None,
                recorded_move: None,
                bot_seat: None,
            })
        }
    }
}

/// The seat list, or [`DEFAULT_SEAT`] at every seat.
///
/// The default needs the root's PLAYER COUNT, which is why it is filled in here
/// and not at parse time — and why this resolves the source once to ask. A
/// seat-count disagreement is left to the driver's own typed `SeatMismatch`
/// rather than pre-empted here, so one authority answers it.
fn seats_or_default(
    seats: Option<Vec<SeatSpec>>,
    asked: &Asked,
    rules: &Rules,
) -> Result<Vec<SeatSpec>, String> {
    if let Some(seats) = seats {
        return Ok(seats);
    }
    let (state, _) = asked
        .source
        .resolve(rules)
        .map_err(|e| format!("{}: no root: {e}", asked.asked_for))?;
    Ok(vec![SeatSpec::Contestant(default_contestant()?); state.players.len()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::arena::{agent_stream_seed, run_arena_game, ArenaAgent, Slot};
    use crate::game::driven::{
        AdjudicationProvenance, AdjudicationSeat, CandidatePlayout, CandidateRow, DrivenOpts,
        RootOrigin, RootSource, SampledWorld, SeatCounters, SESSION_BOT_SEAT,
    };
    use crate::game::driven_families::{
        AdmissionRead, AdmissionVerdict, CounterAnnotation, CounterCheck, ExclusionReason,
        FamilyBlockReport, FamilyLabel, FamilyReadOutcome, FamilyVerdict, PolicyAdmission,
        ReadGrain, ReplicationRead, RootRow, RootVerdict, StrategicClass, MIN_ADMITTED_ROOTS,
        VOID_DISPOSITION,
    };
    use crate::game::legal_moves::get_legal_moves;
    use crate::game::lookahead::RootScore;
    use crate::game::metrics::wilson_95;
    use crate::game::policy::RolloutPolicy;
    use crate::game::record::{state_at, GameRecord, RecordedEvent};
    use crate::game::rules::Rules;
    use crate::game::seeds::{SeedSchedule, WorldSeed};
    use crate::game::semantics::{state_digest, SEMANTICS_REV};
    use crate::types::{GameState, PlayerId};

    /// This task's committed diagnostic master (Phase B, T5). The NAME of the
    /// constructor is the waiver, per `game::seeds`: a fixture cell wants
    /// reproduction, which is exactly the diagnostic case spec §3 carves out.
    const MASTER: u64 = 0x0B01_2026_0818_9995;

    /// The seed a cell-owned registry entry records. Arbitrary and fixed — a
    /// fixture record is not a CLI save, so no derivation produced it.
    const FIXTURE_SEED: u64 = 0x0DEC_1510_0000_0001;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    fn world(index: u64) -> WorldSeed {
        let mut schedule = SeedSchedule::diagnostic(MASTER);
        (0..=index).map(|_| schedule.next_world()).last().unwrap()
    }

    /// Removes the whole temp anchor when the cell leaves, pass or panic.
    struct TempRoot(PathBuf);

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    impl TempRoot {
        fn make(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            let path = std::env::temp_dir()
                .join(format!("yokohama-driven-runner-{tag}-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(&path).expect("the temp anchor is creatable");
            Self(path)
        }
    }

    /// One 2P rung game's record — the cheapest honest record there is.
    fn rung_record(rules: &Rules, seed: WorldSeed) -> GameRecord {
        let mut make = |seat: PlayerId, _players: u8| -> Box<dyn ArenaAgent> {
            Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT)
                .build(rules, agent_stream_seed(seed, seat, Slot(seat.0)))
                .0
        };
        run_arena_game(seed, 2, rules, &mut make, true)
            .unwrap_or_else(|e| panic!("the fixture game must complete: {e}"))
            .record
    }

    fn entry(
        name: &'static str,
        path: &'static str,
        event_index: usize,
        bot_seat: PlayerId,
        recorded_move: Option<Move>,
    ) -> RegistryEntry {
        RegistryEntry {
            name,
            path,
            event_index,
            bot_seat,
            decision_seed: FIXTURE_SEED,
            recorded_move,
            note: "a cell-owned fixture",
        }
    }

    fn descriptor() -> RootDescriptor {
        RootDescriptor {
            origin: RootOrigin::RecordPrefix { record_digest: "record-digest-abc".to_string() },
            event_index: Some(113),
            root_digest: "root-digest-xyz".to_string(),
        }
    }

    fn table_with(winner: Move, ranked: &[(Move, i64)]) -> RootTable {
        RootTable {
            seat: PlayerId(1),
            contestant: Contestant::LOOKAHEAD_V1_DEFAULT,
            echo: Contestant::LOOKAHEAD_V1_DEFAULT.echo(),
            decision_seed: FIXTURE_SEED,
            winner,
            ranked: ranked
                .iter()
                .map(|(mv, value)| RootScore { mv: mv.clone(), value: *value })
                .collect(),
            stats: SearchStats {
                root_candidates: 3,
                nodes: 11,
                handles: 11,
                route_dfs_nodes: 2,
                forced_nodes: 5,
                leaves: 7,
                scored: 29,
                root_scored: 3,
                budget_binds: 0,
                budget_starved: 0,
                placement_cap_binds: 0,
                max_line: 4,
                ply_cap_hits: 0,
                free_extension_steps: 0,
                free_cap_hits: 0,
            },
            world: SampledWorld {
                sampler_seed: 0x0011_2233_4455_6677,
                worlds: 1,
                digest: "world-digest-777".to_string(),
            },
        }
    }

    fn adjudication_with(rows: Vec<CandidateRow>) -> Adjudication {
        Adjudication {
            mover: PlayerId(1),
            root_turn: 17,
            master: WorldSeed(0xABCD_0000_0000_0001),
            replications: 2,
            rows,
            witness: None,
            provenance: AdjudicationProvenance {
                root: descriptor(),
                semantics_rev: SEMANTICS_REV,
                rules_hash: "rules-hash-000".to_string(),
                seats: vec![
                    AdjudicationSeat {
                        seat: 0,
                        requested: "reply:family-weighted".to_string(),
                        echo: Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT).echo(),
                    },
                    AdjudicationSeat {
                        seat: 1,
                        requested: "contestant".to_string(),
                        echo: Contestant::LOOKAHEAD_V1_DEFAULT.echo(),
                    },
                ],
                master: 0xABCD_0000_0000_0001,
                replications: 2,
            },
        }
    }

    fn row(candidate: Move, margins: &[i64]) -> CandidateRow {
        CandidateRow {
            candidate,
            continuation_digest: "continuation-digest".to_string(),
            playouts: margins
                .iter()
                .enumerate()
                .map(|(replication, margin)| CandidatePlayout {
                    replication: replication as u32,
                    world: WorldSeed(0x1000 + replication as u64),
                    agent_seeds: vec![1, 2],
                    margin: *margin,
                    mover_total: 100,
                    best_rival: PlayerId(0),
                    best_rival_total: (100 - margin) as u32,
                    rank: if *margin > 0 { 1 } else { 2 },
                    won: *margin > 0,
                    decisions: 42,
                    turns: 18,
                    final_digest: format!("final-{replication}"),
                    // The runner renders tables; no turn predicate rides one, so
                    // the counter-absence reading is the honest fixture value.
                    turn_verdict: None,
                })
                .collect(),
        }
    }

    // =======================================================================
    // USAGE and the grammar
    // =======================================================================

    /// USAGE names EVERY subcommand and EVERY flag the parser accepts — read off
    /// the parser's own whitelists, so a flag added without a line here fails
    /// rather than shipping undocumented.
    #[test]
    fn the_usage_names_every_subcommand_and_every_flag() {
        for subcommand in SUBCOMMANDS {
            assert!(USAGE.contains(subcommand), "USAGE does not name `{subcommand}`");
        }
        for flags in [LIST_FLAGS, TABLE_FLAGS, RUN_FLAGS, ADJUDICATE_FLAGS] {
            for flag in flags {
                assert!(USAGE.contains(&format!("--{flag}")), "USAGE does not name `--{flag}`");
            }
        }
        // The vocabularies a caller has to type, and the defaults it has to know.
        for token in [
            DEFAULT_SEAT,
            ReplyPolicy::FamilyWeighted.as_str(),
            ReplyPolicy::GreedyAtFinal.as_str(),
            ReplyPolicy::LookaheadAtFinal.as_str(),
            "<path>:<event>",
            "REPRODUCED",
            "NOT CHECKED",
        ] {
            assert!(USAGE.contains(token), "USAGE does not name `{token}`");
        }
        assert!(
            USAGE.contains(&DEFAULT_REPLICATIONS.to_string()),
            "USAGE does not state the default K",
        );
        assert!(USAGE.contains("NO DEFAULT"), "USAGE does not say --master has no default");
        // The two rules a caller cannot infer from a flag list: WHICH arms have
        // a root table at all (every other one is a typed refusal, pinned by
        // `mirror_build`'s own arms), and what a repeated flag does here —
        // which is NOT what `eval_probe` does.
        assert!(USAGE.contains("root table"), "USAGE does not say which arms have a root table");
        assert!(
            USAGE.contains("REPEATED flag"),
            "USAGE does not state the repeated-flag-is-an-error rule",
        );
        assert!(
            USAGE.contains("last-wins"),
            "USAGE does not name the eval_probe rule it departs from",
        );
    }

    /// No subcommand, and an unknown one, are both errors carrying USAGE.
    #[test]
    fn an_absent_or_unknown_subcommand_is_an_error_carrying_the_usage() {
        let empty = parse(&[]).expect_err("no subcommand must refuse");
        assert!(empty.contains(USAGE), "the refusal drops USAGE: {empty}");

        let unknown = parse(&args(&["tabel"])).expect_err("an unknown subcommand must refuse");
        assert!(unknown.contains("tabel"), "the refusal hides what was typed: {unknown}");
        assert!(unknown.contains(USAGE), "the refusal drops USAGE: {unknown}");
    }

    /// An unknown flag refuses at EVERY subcommand — never a silent no-op.
    #[test]
    fn an_unknown_flag_refuses_at_every_subcommand() {
        let lines: [Vec<String>; 4] = [
            args(&["list", "--nope", "1"]),
            args(&["table", "g", "--nope", "1"]),
            args(&["run", "g", "--master", "1", "--nope", "1"]),
            args(&["adjudicate", "g", "--master", "1", "--nope", "1"]),
        ];
        for line in lines {
            let err = match parse(&line) {
                Err(e) => e,
                Ok(command) => panic!("{line:?} must refuse, got {command:?}"),
            };
            assert!(err.contains("nope"), "{line:?}: the refusal hides the flag: {err}");
            assert!(err.contains(USAGE), "{line:?}: the refusal drops USAGE: {err}");
        }
    }

    /// `--replications` is NOT a `run` flag: a single run has nothing to
    /// replicate, and accepting it would silently ignore it.
    #[test]
    fn replications_is_not_a_run_flag() {
        assert!(!RUN_FLAGS.contains(&"replications"), "the whitelist admits it");
        let err = parse(&args(&["run", "g", "--master", "1", "--replications", "4"]))
            .expect_err("--replications on run must refuse");
        assert!(err.contains("replications"), "the refusal hides the flag: {err}");
        // …and it IS an adjudicate flag, so the refusal above is about the
        // subcommand rather than about the spelling.
        assert!(ADJUDICATE_FLAGS.contains(&"replications"));
        parse(&args(&["adjudicate", "g", "--master", "1", "--replications", "4"]))
            .expect("--replications is an adjudicate flag");
    }

    /// A flag without its value refuses — and the honest statement of WHICH
    /// refusal, since the parser does NOT special-case a flag-shaped value.
    ///
    /// Two different failures, both refusals, and the sentences differ because
    /// the failures do: a flag at the END of the line has no next token at all
    /// and refuses for want of a value, while a flag followed by another flag
    /// CONSUMES that next token whatever it looks like and refuses on the VALUE.
    /// What neither one is, is a silent default — which is the property that
    /// matters and the only one this cell is entitled to claim.
    #[test]
    fn a_flag_without_its_value_refuses() {
        let err = parse(&args(&["run", "g", "--master"]))
            .expect_err("a valueless --master must refuse");
        assert!(err.contains("master"), "the refusal hides the flag: {err}");
        assert!(err.contains("needs a value"), "the refusal is not about the absence: {err}");

        // The next token is consumed flag-shaped or not, so THIS one refuses on
        // the value it read rather than on a missing one.
        let err = parse(&args(&["run", "g", "--master", "--seats"]))
            .expect_err("a flag-shaped value must still refuse");
        assert!(err.contains("--seats"), "the refusal hides the value it read: {err}");
        assert!(err.contains("not a u64"), "the refusal is not about the VALUE: {err}");
    }

    /// A REPEATED flag is an ERROR rather than last-wins — the departure from
    /// `eval_probe` that USAGE states, pinned so the statement stays true.
    #[test]
    fn a_repeated_flag_refuses_rather_than_taking_the_last_value() {
        let err = parse(&args(&["run", "g", "--master", "1", "--master", "2"]))
            .expect_err("a repeated --master must refuse");
        assert!(err.contains("master"), "the refusal hides the flag: {err}");
        assert!(err.contains("twice"), "the refusal does not say it was repeated: {err}");
        // ANTI-VACUITY: the same line with ONE --master parses, so the refusal
        // above is about the repetition and not about the line.
        parse(&args(&["run", "g", "--master", "1"])).expect("one --master parses");
    }

    /// Positional arity: `list` takes none, the other three take exactly one.
    #[test]
    fn the_positional_arity_is_checked_per_subcommand() {
        // ANTI-VACUITY first: a parser that refused everything would satisfy
        // every refusal below.
        assert_eq!(parse(&args(&["list"])), Ok(Command::List), "the well-formed line must parse");
        assert!(parse(&args(&["list", "extra"])).is_err(), "list takes no positional");
        assert!(parse(&args(&["table"])).is_err(), "table needs a root");
        assert!(parse(&args(&["table", "a", "b"])).is_err(), "table takes ONE root");
        assert!(parse(&args(&["run", "--master", "1"])).is_err(), "run needs a root");
        assert!(parse(&args(&["adjudicate", "--master", "1"])).is_err(), "adjudicate needs a root");
    }

    /// `run` and `adjudicate` refuse without a master, and the sentence says WHY
    /// (recorded before use by the caller) rather than merely that it is missing.
    #[test]
    fn run_and_adjudicate_refuse_without_a_master() {
        for line in [args(&["run", "g"]), args(&["adjudicate", "g"])] {
            let err = match parse(&line) {
                Err(e) => e,
                Ok(command) => panic!("{line:?} must refuse, got {command:?}"),
            };
            assert!(err.contains("--master"), "{line:?}: {err}");
            assert!(
                err.to_lowercase().contains("record"),
                "{line:?}: the refusal does not say masters are recorded before use: {err}",
            );
        }
    }

    /// The root grammar: a bare name is a fixture, anything with a colon is
    /// `<path>:<event>` split at the LAST colon, and a non-numeric tail refuses.
    #[test]
    fn the_root_grammar_splits_a_path_at_the_last_colon() {
        assert_eq!(
            parse_root_ref("g1-e113-tiemass").expect("a bare name is a fixture"),
            RootRef::Fixture("g1-e113-tiemass".to_string()),
        );
        assert_eq!(
            parse_root_ref("/tmp/a:b/game.json:113").expect("a path:event parses"),
            RootRef::Path { path: PathBuf::from("/tmp/a:b/game.json"), event_index: 113 },
        );
        let err = parse_root_ref("game.json:notanumber").expect_err("a non-numeric tail refuses");
        assert!(err.contains("notanumber"), "the refusal hides the tail: {err}");
    }

    /// Seat specs: the reply menu's three bare names, else the contestant
    /// grammar, and unknown is an error.
    #[test]
    fn seat_specs_read_the_reply_menu_then_the_contestant_grammar() {
        assert_eq!(
            parse_seat("family-weighted").expect("the menu slot parses"),
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
        );
        assert_eq!(
            parse_seat("greedy-at-final").expect("the menu slot parses"),
            SeatSpec::Reply(ReplyPolicy::GreedyAtFinal),
        );
        assert_eq!(
            parse_seat("lookahead-at-final").expect("the menu slot parses"),
            SeatSpec::Reply(ReplyPolicy::LookaheadAtFinal),
        );
        // A KNOBBED rung is the contestant grammar, not the menu slot — and both
        // resolve to a rung, so only the recorded `requested` label differs.
        let knobbed = parse_seat("family-weighted:p=50000,k=5").expect("the knobbed rung parses");
        assert!(matches!(knobbed, SeatSpec::Contestant(Contestant::Policy(_))));
        assert_eq!(
            parse_seat("lookahead-v1:b=8").expect("the contestant grammar parses"),
            SeatSpec::Contestant(
                Contestant::from_str_name("lookahead-v1:b=8").expect("the grammar accepts it")
            ),
        );
        let err = parse_seat("nosuchseat").expect_err("an unknown seat name refuses");
        assert!(err.contains("nosuchseat"), "the refusal hides the name: {err}");

        assert_eq!(
            parse_seats("family-weighted,lookahead-v1").expect("a two-seat list parses").len(),
            2,
        );
        assert!(parse_seats("").is_err(), "an empty seat list is not a seat list");
        assert!(parse_seats("family-weighted,nope").is_err(), "one bad seat refuses the list");
    }

    /// The default seat and the default contestant are THE SHIPPED
    /// lookahead-v1, spelled through the grammar rather than through a second
    /// const this module would own.
    #[test]
    fn the_default_seat_is_the_shipped_lookahead_v1_through_the_grammar() {
        assert_eq!(
            Contestant::from_str_name(DEFAULT_SEAT).expect("the default spells a contestant"),
            Contestant::LOOKAHEAD_V1_DEFAULT,
        );
        match parse(&args(&["table", "g"])).expect("table parses without --contestant") {
            Command::Table { contestant, decision_seed, .. } => {
                assert_eq!(contestant, Contestant::LOOKAHEAD_V1_DEFAULT);
                assert_eq!(decision_seed, None, "the seed defaults at EXECUTION, not at parse");
            }
            other => panic!("expected a Table command, got {other:?}"),
        }
        match parse(&args(&["adjudicate", "g", "--master", "1"]))
            .expect("adjudicate parses without --replications")
        {
            Command::Adjudicate { replications, seats, .. } => {
                assert_eq!(replications, DEFAULT_REPLICATIONS);
                assert_eq!(seats, None, "the seat list defaults at EXECUTION (it needs a count)");
            }
            other => panic!("expected an Adjudicate command, got {other:?}"),
        }
    }

    /// Masters and seeds parse as `eval_probe` parses them.
    #[test]
    fn numbers_parse_as_hex_or_decimal_with_underscores() {
        assert_eq!(parse_u64("0x15e8_0120_dfcb_f8c3"), Ok(0x15e8_0120_dfcb_f8c3));
        assert_eq!(parse_u64("0X10"), Ok(16));
        assert_eq!(parse_u64("1_000"), Ok(1000));
        assert!(parse_u64("zzz").is_err());
    }

    // =======================================================================
    // Rendering
    // =======================================================================

    /// The counter block names ALL FIFTEEN `SearchStats` fields.
    ///
    /// The names come off [`counter_rows`], whose body destructures
    /// `SearchStats` exhaustively — so a sixteenth counter is a COMPILE error
    /// there rather than a silently-unprinted field here.
    #[test]
    fn the_table_render_names_all_fifteen_search_counters() {
        let table = table_with(
            Move::BuildShophouse { slot: 0 },
            &[(Move::BuildShophouse { slot: 0 }, 116), (Move::BuildShophouse { slot: 1 }, 115)],
        );
        let root = descriptor();
        let rendered = render_root_table(
            &table,
            &TableContext {
                asked_for: "g1-e113-tiemass",
                root: &root,
                recorded_move: None,
                recorded_seed: None,
            },
        );
        let rows = counter_rows(&table.stats);
        assert_eq!(rows.len(), 15, "the counter set is not fifteen");
        for (name, value) in rows {
            assert!(!name.is_empty(), "an unnamed counter row");
            assert!(rendered.contains(name), "the render omits counter `{name}`: {rendered}");
            let _ = value;
        }
        // The values, not merely the names: a block of fifteen zeros would pass
        // a names-only check on any stats at all.
        assert!(rendered.contains("29"), "the render omits `scored`'s value: {rendered}");
    }

    /// The shape fields the separator reads, and the winner.
    #[test]
    fn the_table_render_carries_the_ranking_shape_and_the_world() {
        let table = table_with(
            Move::BuildShophouse { slot: 0 },
            &[
                (Move::BuildShophouse { slot: 0 }, 116),
                (Move::BuildShophouse { slot: 1 }, 116),
                (Move::BuildShophouse { slot: 2 }, 115),
                (Move::BuildShophouse { slot: 3 }, 12),
            ],
        );
        let root = descriptor();
        let rendered = render_root_table(
            &table,
            &TableContext {
                asked_for: "fixture",
                root: &root,
                recorded_move: None,
                recorded_seed: None,
            },
        );
        for token in [
            "fixture",
            &root.root_digest,
            &table.echo,
            &format!("{:#018x}", table.decision_seed),
            &format!("{:#018x}", table.world.sampler_seed),
            &table.world.digest,
        ] {
            assert!(rendered.contains(token), "the render omits `{token}`: {rendered}");
        }
        assert!(rendered.contains("113"), "the render omits the event index: {rendered}");
        // candidates 4, tied-at-top 2, top band (<=1 point) 3, distinct 3.
        assert_eq!((table.candidates(), table.tied_at_top()), (4, 2));
        assert_eq!((table.top_band(), table.distinct_values()), (3, 3));
        for token in ["4", "2", "3"] {
            assert!(rendered.contains(token), "the render omits shape figure {token}: {rendered}");
        }
    }

    /// The ranked-head BLOCK of a rendered table: the lines between the
    /// `ranked head` header and the blank line that closes it.
    fn ranked_head_rows(rendered: &str) -> Vec<&str> {
        rendered
            .lines()
            .skip_while(|line| !line.contains("ranked head"))
            .skip(1)
            .take_while(|line| !line.trim().is_empty())
            .collect()
    }

    /// The ranked head is BEST FIRST and its first row is THE WINNER.
    ///
    /// The two facts a reader takes off that block without being told, and
    /// neither is checked by a `contains` of a move name: a head rendered in
    /// reverse still contains every move, every value and every margin, and
    /// still prints a winner line above it — it just answers the separator's
    /// question upside down, which is the whole reading the block exists for.
    #[test]
    fn the_ranked_head_leads_with_the_winner_and_its_margins_never_rise() {
        let winner = Move::BuildShophouse { slot: 0 };
        let table = table_with(
            winner.clone(),
            &[
                (winner.clone(), 116),
                (Move::BuildShophouse { slot: 1 }, 116),
                (Move::BuildShophouse { slot: 2 }, 115),
                (Move::BuildShophouse { slot: 3 }, 12),
            ],
        );
        let root = descriptor();
        let rendered = render_root_table(
            &table,
            &TableContext {
                asked_for: "fixture",
                root: &root,
                recorded_move: None,
                recorded_seed: None,
            },
        );
        let head = ranked_head_rows(&rendered);
        assert_eq!(head.len(), table.ranked.len(), "the head is not the ranking:\n{rendered}");

        // ROW 1 IS THE WINNER — the move the table's own `winner` field names,
        // not merely a move that appears somewhere in the block.
        assert!(
            head[0].ends_with(&format!("{:?}", table.winner)),
            "row 1 is not the winner {:?}: {}",
            table.winner,
            head[0],
        );

        // …and the printed MARGINS never rise going down (best first).
        let margins: Vec<i64> = head
            .iter()
            .map(|row| {
                let token = row.split_whitespace().nth(2).unwrap_or_else(|| {
                    panic!("a head row has no margin column: {row}")
                });
                token.parse().unwrap_or_else(|e| panic!("margin {token:?} does not parse: {e}"))
            })
            .collect();
        assert!(
            margins.windows(2).all(|pair| pair[0] >= pair[1]),
            "the head is not best-first: {margins:?}",
        );
        // ANTI-VACUITY: a CONSTANT margin vector is non-increasing too, so the
        // fixture has to spread — and the spread is what a reversal inverts.
        assert!(
            margins.first() > margins.last(),
            "the fixture does not discriminate direction: {margins:?}",
        );
    }

    /// The reproduction line: REPRODUCED when the winner is the recorded move,
    /// DIVERGED when it is not, NOT CHECKED when there is nothing to compare —
    /// plus the OVERRIDDEN qualifier when the seed is not the entry's own.
    #[test]
    fn the_reproduction_line_reads_reproduced_diverged_or_not_checked() {
        let winner = Move::BuildShophouse { slot: 0 };
        let other = Move::BuildShophouse { slot: 7 };
        let table = table_with(winner.clone(), &[(winner.clone(), 5), (other.clone(), 4)]);
        let root = descriptor();
        let ctx = |recorded_move: Option<&Move>, recorded_seed: Option<u64>| {
            render_root_table(
                &table,
                &TableContext {
                    asked_for: "fixture",
                    root: &root,
                    recorded_move,
                    recorded_seed,
                },
            )
        };

        let reproduced = ctx(Some(&winner), Some(FIXTURE_SEED));
        assert!(reproduced.contains("REPRODUCED"), "no reproduced verdict: {reproduced}");
        assert!(!reproduced.contains("DIVERGED"), "both verdicts printed: {reproduced}");
        assert!(!reproduced.contains("OVERRIDDEN"), "a matching seat is not an override");

        let diverged = ctx(Some(&other), Some(FIXTURE_SEED));
        assert!(diverged.contains("DIVERGED"), "no diverged verdict: {diverged}");
        assert!(!diverged.contains("REPRODUCED"), "both verdicts printed: {diverged}");

        let unchecked = ctx(None, None);
        assert!(unchecked.contains("NOT CHECKED"), "no not-checked verdict: {unchecked}");
        assert!(!unchecked.contains("REPRODUCED"), "a path root claims a reproduction");

        // An OVERRIDDEN seed is still compared, and the qualifier says the
        // comparison is no longer the entry's own b=4 reproduction check.
        let overridden = ctx(Some(&winner), Some(FIXTURE_SEED ^ 0xFF));
        assert!(overridden.contains("OVERRIDDEN"), "no override qualifier: {overridden}");
    }

    /// Rendering is a function of its inputs — the property that lets a report
    /// be quoted.
    #[test]
    fn rendering_is_deterministic_at_fixed_inputs() {
        let table = table_with(
            Move::BuildShophouse { slot: 0 },
            &[(Move::BuildShophouse { slot: 0 }, 3), (Move::BuildShophouse { slot: 1 }, 1)],
        );
        let root = descriptor();
        let ctx = TableContext {
            asked_for: "fixture",
            root: &root,
            recorded_move: None,
            recorded_seed: None,
        };
        let once = render_root_table(&table, &ctx);
        // ANTI-VACUITY: two empty strings are equal, so an unimplemented
        // renderer would satisfy the identity below without rendering anything.
        assert!(once.lines().count() > 5, "the table render is a stub: {once:?}");
        assert_eq!(once, render_root_table(&table, &ctx));

        let adjudication = adjudication_with(vec![
            row(Move::BuildShophouse { slot: 0 }, &[3, 3]),
            row(Move::BuildShophouse { slot: 1 }, &[1, 5]),
        ]);
        let once = render_adjudication(&adjudication, "fixture");
        assert!(once.lines().count() > 5, "the adjudication render is a stub: {once:?}");
        assert_eq!(once, render_adjudication(&adjudication, "fixture"));
    }

    /// The adjudication report OPENS with the robustness-read label, verbatim
    /// (T4 NOTE-2: the label rides stdout, not only the docs).
    #[test]
    fn the_adjudication_render_opens_with_the_robustness_read_verbatim() {
        let adjudication = adjudication_with(vec![row(Move::BuildShophouse { slot: 0 }, &[3, 3])]);
        let rendered = render_adjudication(&adjudication, "fixture");
        assert_eq!(
            rendered.lines().next(),
            Some(driven::ROBUSTNESS_READ),
            "the first line is not the robustness read",
        );
        assert!(
            rendered.contains(driven::ROBUSTNESS_READ),
            "the label is not present verbatim: {rendered}",
        );
    }

    /// One row per candidate, with margins, mean/min/max, wins — and the
    /// all-candidates-equal fact stated rather than left to the reader.
    #[test]
    fn the_adjudication_render_states_every_candidate_and_the_equality_fact() {
        let equal = adjudication_with(vec![
            row(Move::BuildShophouse { slot: 0 }, &[3, 3]),
            row(Move::BuildShophouse { slot: 1 }, &[3, 3]),
        ]);
        assert!(equal.all_candidates_equal());
        let rendered = render_adjudication(&equal, "fixture");
        assert!(rendered.contains("all candidates equal"), "no equality line: {rendered}");
        assert!(rendered.contains("true"), "the equality fact is not stated: {rendered}");
        assert!(rendered.contains("BuildShophouse"), "no candidate rows: {rendered}");
        // The reproduction digest rides the report as a FOLD of itself (N-5) —
        // the string is `candidates × K` long and the report has to be
        // quotable. `digest_fold`'s own cell pins the fold's properties.
        assert!(
            rendered.contains(&digest_fold(&equal.digest())),
            "no reproduction digest: {rendered}",
        );
        assert!(rendered.contains("p1"), "the mover is not named: {rendered}");

        let unequal = adjudication_with(vec![
            row(Move::BuildShophouse { slot: 0 }, &[3, 3]),
            row(Move::BuildShophouse { slot: 1 }, &[1, 5]),
        ]);
        assert!(!unequal.all_candidates_equal());
        let rendered = render_adjudication(&unequal, "fixture");
        assert!(rendered.contains("false"), "the equality fact is not stated: {rendered}");
        // mean of [1, 5] is 3.00 while the margins differ — the elementwise
        // reading is the one that matters, and both must be visible.
        assert!(rendered.contains("+5"), "the max margin is not printed: {rendered}");
    }

    /// The report's `digest` line, which there is exactly one of.
    fn digest_line(rendered: &str) -> &str {
        let mut found = rendered.lines().filter(|line| line.trim_start().starts_with("digest"));
        let line = found.next().unwrap_or_else(|| panic!("no digest line:\n{rendered}"));
        assert_eq!(found.next(), None, "more than one digest line:\n{rendered}");
        line
    }

    /// The adjudication's digest line is a SHORT, STABLE, LABELLED fold that
    /// still moves with the table (N-5).
    ///
    /// The line used to print `Adjudication::digest()` raw, which is
    /// `candidates × K` long — about 15 KB on a real 139-candidate
    /// two-replication run, i.e. a reproduction check nobody can quote and
    /// therefore nobody performs. Four properties make the fold a replacement
    /// rather than a deletion: it is ONE line, it is short enough to paste, it
    /// reproduces, and it CHANGES when the table does.
    #[test]
    fn the_adjudication_digest_line_is_a_short_stable_fold_that_moves_with_a_margin() {
        let adjudication = adjudication_with(vec![
            row(Move::BuildShophouse { slot: 0 }, &[3, 3]),
            row(Move::BuildShophouse { slot: 1 }, &[1, 5]),
        ]);
        let rendered = render_adjudication(&adjudication, "fixture");
        let line = digest_line(&rendered);

        assert!(line.len() < 100, "the digest line is {} chars, not quotable: {line}", line.len());
        // It SAYS what it is: a hash, and of what.
        assert!(line.contains("fnv1a64:"), "the line does not label its fold: {line}");
        assert!(
            line.contains("reproduction string"),
            "the line does not say what was folded: {line}",
        );
        assert!(
            line.contains(&digest_fold(&adjudication.digest())),
            "the line is not the fold of THIS table's digest: {line}",
        );
        // The raw string is GONE, and it is genuinely the longer thing — so the
        // shortening is a shortening rather than a coincidence of this fixture.
        let full = adjudication.digest();
        assert!(!line.contains(&full), "the full digest is still printed: {line}");
        assert!(full.len() > line.len(), "anti-vacuity: the raw digest is not the longer form");

        // It reproduces across two renders…
        assert_eq!(line, digest_line(&render_adjudication(&adjudication, "fixture")));

        // …and it MOVES when one replication's margin moves. A digest line that
        // did not would be a decoration rather than a reproduction check.
        let moved = adjudication_with(vec![
            row(Move::BuildShophouse { slot: 0 }, &[3, 3]),
            row(Move::BuildShophouse { slot: 1 }, &[1, 6]),
        ]);
        assert_ne!(
            line,
            digest_line(&render_adjudication(&moved, "fixture")),
            "one flipped margin left the digest line unmoved",
        );
    }

    /// The adjudication table's rows as `(row line, margins line)` pairs —
    /// which also pins that a row IS two lines, since the second one carries
    /// the margin vector the first one summarizes.
    fn adjudication_rows(rendered: &str) -> Vec<(&str, &str)> {
        let mut lines = rendered
            .lines()
            .skip_while(|line| !line.contains("candidate / margins"))
            .skip(1);
        let mut rows = Vec::new();
        while let Some(row_line) = lines.next() {
            if row_line.trim().is_empty() {
                break;
            }
            let margins_line = lines
                .next()
                .unwrap_or_else(|| panic!("a row line has no margins line: {row_line}"));
            assert!(
                margins_line.trim_start().starts_with("margins ["),
                "a row's second line is not its margins: {margins_line}",
            );
            rows.push((row_line, margins_line));
        }
        rows
    }

    /// Every printed `min` sits at or below its own row's `max`, and BOTH are
    /// margins that row actually recorded.
    ///
    /// The two columns are adjacent, same width, same `{:+}` formatting and one
    /// field name apart — a transposition compiles clean and reads plausible,
    /// and every mean/wins figure around it stays right. What catches it is the
    /// ORDER relation plus membership in the row's own margin list; a
    /// same-margin row cannot catch it at all, which is why the fixture below
    /// spreads and why the cell asserts that it does.
    #[test]
    fn every_adjudication_row_prints_a_min_at_or_below_its_max_and_both_are_margins() {
        let adjudication = adjudication_with(vec![
            row(Move::BuildShophouse { slot: 0 }, &[3, 3]),
            row(Move::BuildShophouse { slot: 1 }, &[1, 5]),
            row(Move::BuildShophouse { slot: 2 }, &[-4, 2]),
        ]);
        let rendered = render_adjudication(&adjudication, "fixture");
        let rows = adjudication_rows(&rendered);
        assert_eq!(rows.len(), adjudication.rows.len(), "not one row per candidate:\n{rendered}");

        let mut spread = 0;
        for (index, (row_line, margins_line)) in rows.iter().enumerate() {
            let cell = |position: usize| -> i64 {
                let token = row_line.split_whitespace().nth(position).unwrap_or_else(|| {
                    panic!("row {index} has no column {position}: {row_line}")
                });
                token
                    .parse()
                    .unwrap_or_else(|e| panic!("row {index} column {position} {token:?}: {e}"))
            };
            // Columns: #, mean, MIN, MAX, wins, k.
            let (min, max) = (cell(2), cell(3));
            let printed: Vec<i64> = margins_line
                .split_once('[')
                .and_then(|(_, tail)| tail.split_once(']'))
                .unwrap_or_else(|| panic!("row {index} has no margin list: {margins_line}"))
                .0
                .split(',')
                .map(|m| {
                    m.trim().parse().unwrap_or_else(|e| panic!("margin {m:?} in row {index}: {e}"))
                })
                .collect();

            assert_eq!(printed, adjudication.rows[index].margins(), "row {index}'s margins");
            assert!(min <= max, "row {index} prints min {min} above max {max}: {row_line}");
            assert!(printed.contains(&min), "row {index}'s min {min} is not one of {printed:?}");
            assert!(printed.contains(&max), "row {index}'s max {max} is not one of {printed:?}");
            if min != max {
                spread += 1;
            }
        }
        // ANTI-VACUITY: a row whose margins are all equal satisfies min <= max
        // and both-are-margins under a TRANSPOSED render too.
        assert!(spread > 0, "no row spreads, so the fixture cannot see a transposition");
    }

    /// The `list` line that NAMES one entry — the line its presence marker is
    /// printed on.
    ///
    /// A per-entry assertion has to FIND this line rather than search the
    /// document, and the reason is concrete: the report's own header sentence
    /// carries the word MISSING ("a MISSING record is a MISS and never a
    /// failure"), so `rendered.contains("MISSING")` is satisfied by a render
    /// that marked every entry PRESENT — including one that inverted the test
    /// outright.
    fn entry_line<'a>(rendered: &'a str, name: &str) -> &'a str {
        let mut named =
            rendered.lines().filter(|line| line.trim_start().starts_with(name));
        let line = named
            .next()
            .unwrap_or_else(|| panic!("no line names `{name}`:\n{rendered}"));
        assert_eq!(named.next(), None, "more than one line names `{name}`:\n{rendered}");
        line
    }

    /// The `list` report marks EACH record PRESENT or MISSING on that entry's
    /// own line against the anchor it was given, and NEVER errors on absence.
    #[test]
    fn the_list_report_marks_presence_and_never_errors_on_absence() {
        let anchor = TempRoot::make("list");
        std::fs::create_dir_all(anchor.0.join("here")).expect("the subdir is creatable");
        std::fs::write(anchor.0.join("here/present.json"), "{}").expect("the file is writable");

        let entries = [
            entry("present-one", "here/present.json", 3, PlayerId(1), None),
            entry("absent-one", "here/absent.json", 4, PlayerId(0), None),
        ];
        let rendered = render_list(&entries, &anchor.0);
        // PER ENTRY, on the bracketed marker: the present record's own line says
        // PRESENT and only PRESENT, the absent one's says MISSING and only
        // MISSING. An inverted marker passes every document-level `contains`
        // and fails here.
        let present = entry_line(&rendered, "present-one");
        assert!(present.contains("[PRESENT]"), "the present record is not marked: {present}");
        assert!(!present.contains("[MISSING]"), "the present record is also marked absent: {present}");
        let absent = entry_line(&rendered, "absent-one");
        assert!(absent.contains("[MISSING]"), "the absent record is not marked: {absent}");
        assert!(!absent.contains("[PRESENT]"), "the absent record is also marked present: {absent}");

        assert!(rendered.contains("a cell-owned fixture"), "the note is dropped: {rendered}");
        assert!(rendered.contains("here/present.json"), "the path is dropped: {rendered}");
        assert!(
            rendered.contains(&format!("{FIXTURE_SEED:#018x}")),
            "the recorded seed is dropped: {rendered}",
        );

        // Through the command, with an anchor that holds NOTHING: still Ok, and
        // NO line claims a present record. Stated as "no line" rather than as
        // "the report says MISSING" for the same vacuity reason.
        let empty = TempRoot::make("list-empty");
        let out = run_with(&args(&["list"]), &entries, &empty.0).expect("list never errors");
        assert!(
            !out.lines().any(|line| line.contains("[PRESENT]")),
            "an anchor holding nothing marked a record PRESENT: {out}",
        );
        for name in ["present-one", "absent-one"] {
            let line = entry_line(&out, name);
            assert!(line.contains("[MISSING]"), "`{name}` is not marked absent: {line}");
        }
    }

    /// The SHIPPED registry renders — every entry named, nothing panicking, and
    /// no dependence on whether this checkout carries the gitignored saves.
    #[test]
    fn the_shipped_registry_lists_every_entry() {
        let rendered = run(&args(&["list"])).expect("list never errors");
        for entry in driven::registry() {
            assert!(rendered.contains(entry.name), "the list omits {}: {rendered}", entry.name);
            assert!(rendered.contains(entry.path), "the list omits {}'s path", entry.name);
        }
    }

    // =======================================================================
    // Execution, on cell-owned records
    // =======================================================================

    /// Walk `record`'s positions and return the first `(event_index, state)`
    /// whose decision satisfies `wanted`.
    fn find_position(
        record: &GameRecord,
        rules: &Rules,
        mut wanted: impl FnMut(&GameState, &Move) -> bool,
    ) -> Option<(usize, GameState)> {
        let mut state = state_at(record, 0, rules).expect("position 0 replays");
        for (position, event) in record.events.iter().enumerate() {
            if let RecordedEvent::Move(mv) = event {
                if wanted(&state, mv) {
                    return Some((position, state));
                }
            }
            state = crate::game::record::replay_from(&state, std::slice::from_ref(event), rules)
                .expect("the fixture record replays");
        }
        None
    }

    /// A `path:event` root REFUSES without `--decision-seed`, and the sentence
    /// says why (no seed is recorded for such a root).
    #[test]
    fn a_path_root_requires_a_decision_seed() {
        let rules = Rules::standard();
        let anchor = TempRoot::make("path-seed");
        let record = rung_record(&rules, world(1));
        let file = anchor.0.join("fixture.json");
        std::fs::write(&file, serde_json::to_string(&record).expect("the record serializes"))
            .expect("the fixture is writable");

        let spec = format!("{}:3", file.display());
        let err = run_with(&args(&["table", &spec]), &[], &anchor.0)
            .expect_err("a path root without a seed must refuse");
        assert!(err.contains("--decision-seed"), "the refusal hides the flag: {err}");
        assert!(
            err.to_lowercase().contains("recorded"),
            "the refusal does not say no seed is recorded: {err}",
        );
    }

    /// The whole `table` path on a CELL-OWNED record: the registry default seed
    /// is the entry's own, and the reproduction line reads REPRODUCED at a
    /// FORCED decision (where every honest contestant plays the one legal move)
    /// and DIVERGED where the rung and the search disagree.
    #[test]
    fn the_table_command_reproduces_a_forced_decision_and_reports_a_divergence() {
        let rules = Rules::standard();
        let anchor = TempRoot::make("table");
        let record = rung_record(&rules, world(2));
        std::fs::write(
            anchor.0.join("fixture.json"),
            serde_json::to_string(&record).expect("the record serializes"),
        )
        .expect("the fixture is writable");

        // A FORCED decision: one legal move, so the recorded rung move and the
        // search's pick are the same move by construction.
        let (forced_index, forced_state) =
            find_position(&record, &rules, |state, mv| {
                let legal = get_legal_moves(state, &rules);
                legal.len() == 1 && legal[0] == *mv
            })
            .expect("a 2P game has a forced decision");
        let forced_move = match &record.events[forced_index] {
            RecordedEvent::Move(mv) => mv.clone(),
            other => panic!("event {forced_index} is {other:?}"),
        };
        let entries = [entry(
            "forced",
            "fixture.json",
            forced_index,
            forced_state.current_player,
            Some(forced_move),
        )];
        let out = run_with(&args(&["table", "forced"]), &entries, &anchor.0)
            .expect("the forced decision tabulates");
        assert!(out.contains("REPRODUCED"), "a forced decision must reproduce: {out}");
        assert!(
            out.contains(&format!("{FIXTURE_SEED:#018x}")),
            "the entry's recorded seed is not the default: {out}",
        );
        assert!(!out.contains("OVERRIDDEN"), "the default seed is not an override: {out}");

        // An explicit seed that is NOT the entry's is still compared, under the
        // override qualifier.
        let out = run_with(
            &args(&["table", "forced", "--decision-seed", "0x1"]),
            &entries,
            &anchor.0,
        )
        .expect("an explicit seed tabulates");
        assert!(out.contains("OVERRIDDEN"), "an explicit seed is an override: {out}");

        // A BRANCHING decision where the rung's recorded move is not the
        // search's argmax. Bounded so a pathological record cannot hang the
        // suite; a scan that finds none is a finding, not a pass.
        let mut tried = 0;
        let mut diverged = None;
        let mut state = state_at(&record, 0, &rules).expect("position 0 replays");
        for (position, event) in record.events.iter().enumerate() {
            if let RecordedEvent::Move(mv) = event {
                if tried < 12 && get_legal_moves(&state, &rules).len() > 1 {
                    tried += 1;
                    let entries = [entry(
                        "branching",
                        "fixture.json",
                        position,
                        state.current_player,
                        Some(mv.clone()),
                    )];
                    let out = run_with(&args(&["table", "branching"]), &entries, &anchor.0)
                        .expect("a branching decision tabulates");
                    if out.contains("DIVERGED") {
                        diverged = Some(out);
                        break;
                    }
                }
            }
            state = crate::game::record::replay_from(&state, std::slice::from_ref(event), &rules)
                .expect("the fixture record replays");
        }
        let out = diverged.expect("no rung move differed from the search's pick in 12 tries");
        assert!(!out.contains("REPRODUCED"), "both verdicts printed: {out}");
    }

    /// An unknown fixture name refuses and NAMES the entries that exist.
    #[test]
    fn an_unknown_fixture_name_refuses_naming_what_exists() {
        let anchor = TempRoot::make("unknown");
        let entries = [entry("known-one", "nothing.json", 1, PlayerId(0), None)];
        let err = run_with(&args(&["table", "nope"]), &entries, &anchor.0)
            .expect_err("an unknown fixture must refuse");
        assert!(err.contains("nope"), "the refusal hides what was asked for: {err}");
        assert!(err.contains("known-one"), "the refusal does not name what exists: {err}");
    }

    /// A `table` on an entry whose recorded seat is NOT the mover refuses: the
    /// decision analyzed is the mover's, and an entry claiming otherwise is
    /// pointed at the wrong event.
    #[test]
    fn a_table_refuses_an_entry_whose_recorded_seat_is_not_the_mover() {
        let rules = Rules::standard();
        let anchor = TempRoot::make("seat");
        let record = rung_record(&rules, world(3));
        std::fs::write(
            anchor.0.join("fixture.json"),
            serde_json::to_string(&record).expect("the record serializes"),
        )
        .expect("the fixture is writable");
        let (index, state) =
            find_position(&record, &rules, |_, _| true).expect("the record carries a move");
        let wrong = PlayerId(1 - state.current_player.0);
        let entries = [entry("wrong-seat", "fixture.json", index, wrong, None)];
        let err = run_with(&args(&["table", "wrong-seat"]), &entries, &anchor.0)
            .expect_err("a mis-seated entry must refuse");
        assert!(err.contains("wrong-seat"), "the refusal hides the entry: {err}");
    }

    /// A MISSING record is a typed miss the runner reports rather than a panic.
    #[test]
    fn a_missing_record_is_reported_not_panicked() {
        let anchor = TempRoot::make("missing");
        let entries = [entry("absent", "nothing.json", 1, PlayerId(0), None)];
        let err = run_with(&args(&["table", "absent", "--decision-seed", "1"]), &entries, &anchor.0)
            .expect_err("an absent record must refuse");
        assert!(err.contains("nothing.json"), "the refusal hides the path: {err}");
    }

    /// The `run` report carries provenance, seats, the result and the digest.
    ///
    /// Rung seats deliberately (design §6's wall discipline): the property under
    /// test is the REPORT, and a debug lookahead game costs seconds.
    #[test]
    fn the_run_report_carries_provenance_seats_result_and_digest() {
        let rules = Rules::standard();
        let anchor = TempRoot::make("run");
        let record = rung_record(&rules, world(4));
        std::fs::write(
            anchor.0.join("fixture.json"),
            serde_json::to_string(&record).expect("the record serializes"),
        )
        .expect("the fixture is writable");
        // Near the END of the game, so the remainder is short.
        let index = record.events.len() - 3;
        let entries = [entry("tail", "fixture.json", index, SESSION_BOT_SEAT, None)];

        let out = run_with(
            &args(&[
                "run",
                "tail",
                "--master",
                "0x0B01_2026_0818_0001",
                "--seats",
                "family-weighted,family-weighted",
            ]),
            &entries,
            &anchor.0,
        )
        .expect("the tail run completes");

        for token in ["record-prefix", "winner", "decisions", "driven-v1", "family-weighted"] {
            assert!(out.contains(token), "the run report omits `{token}`: {out}");
        }
        assert!(out.contains(&format!("rev={SEMANTICS_REV}")), "no semantics stamp: {out}");
        assert!(out.contains(&rules.content_hash()), "no rules stamp: {out}");
        assert!(out.contains(&format!("{:#018x}", 0x0B01_2026_0818_0001_u64)), "no master: {out}");

        // The same command twice is the same report — determinism through the
        // whole execution path, not only through the renderer.
        let again = run_with(
            &args(&[
                "run",
                "tail",
                "--master",
                "0x0B01_2026_0818_0001",
                "--seats",
                "family-weighted,family-weighted",
            ]),
            &entries,
            &anchor.0,
        )
        .expect("the tail run completes");
        assert_eq!(out, again, "two identical run commands produced different reports");
    }

    /// One `key=value` line, split into its fields.
    ///
    /// Exact KEYS rather than substrings, because the counter line carries both
    /// `scored=` and `root_scored=`: a `contains("scored=3")` check is satisfied
    /// by the wrong field, and a render that swapped the two would pass it.
    fn fields(line: &str) -> BTreeMap<&str, &str> {
        line.split_whitespace().filter_map(|token| token.split_once('=')).collect()
    }

    /// The run report's per-seat counter line says WHAT THE OUTCOME SAYS, field
    /// by field — and for a LOOKAHEAD seat the explained and scored fields are
    /// NUMERIC, never the `-` the legend reserves for "this contestant has no
    /// such quantity".
    ///
    /// Seated with the shipped lookahead-v1 deliberately: every other run cell
    /// in this file is rung-seated (design §6's wall discipline), and a rung
    /// prints `-` at all seven of these legitimately — so a render that hard-
    /// coded the dash, or read a different seat's counters, would pass every one
    /// of them. The root is near the end of a played game so a debug cell can
    /// afford two uncapped searches.
    #[test]
    fn the_run_report_counter_line_agrees_with_a_lookahead_seats_counters() {
        let rules = Rules::standard();
        let record = rung_record(&rules, world(9));
        let event_index = record.events.len() - 4;
        let source = RootSource::RecordPrefix { record, event_index };
        let (state, _) = source.resolve(&rules).expect("the prefix resolves");
        let seats =
            vec![SeatSpec::Contestant(Contestant::LOOKAHEAD_V1_DEFAULT); state.players.len()];
        let outcome = driven::drive_rooted(
            &source,
            &seats,
            &rules,
            world(10),
            DrivenOpts::default(),
        )
        .expect("the tail run completes");
        let rendered = render_outcome(&outcome, "constructed");

        let mut scored_anywhere = 0;
        for seat in &outcome.seats {
            let prefix = format!("p{}  moves=", seat.seat.0);
            let line = rendered
                .lines()
                .find(|line| line.trim_start().starts_with(&prefix))
                .unwrap_or_else(|| panic!("no counter line for p{}:\n{rendered}", seat.seat.0));
            let printed = fields(line);
            let field = |name: &str| -> &str {
                printed
                    .get(name)
                    .unwrap_or_else(|| panic!("the counter line has no `{name}`: {line}"))
            };
            let counters = &seat.counters;

            // A lookahead seat HAS an explanation path and a scored budget. Said
            // first, because "never `-`" is only a claim about the RENDER once
            // the seat is known to have the quantity.
            assert!(
                counters.explained.is_some() && counters.candidates_scored.is_some(),
                "p{} is not an explaining, scoring seat — this cell is not seated as it claims",
                seat.seat.0,
            );

            // Every counter, as the legend says it renders: the value where the
            // seat has the quantity, `-` where it does not.
            for (name, counter) in [
                ("explained", counters.explained),
                ("scored", counters.candidates_scored),
                ("root_scored", counters.root_scored),
                ("binds", counters.budget_binds),
                ("starved", counters.budget_starved_decisions),
                ("ply_cap", counters.ply_cap_hits),
                ("free_cap", counters.free_cap_hits),
            ] {
                assert_eq!(
                    field(name),
                    counter.map_or_else(|| "-".to_string(), |value| value.to_string()),
                    "p{}'s `{name}` field disagrees with its counters: {line}",
                    seat.seat.0,
                );
            }
            assert_eq!(
                field("moves"),
                seat.applied_moves.to_string(),
                "p{}'s applied moves disagree: {line}",
                seat.seat.0,
            );
            assert_ne!(field("explained"), "-", "a lookahead seat's explained read absent: {line}");
            assert_ne!(field("scored"), "-", "a lookahead seat's scored read absent: {line}");

            scored_anywhere += counters.candidates_scored.unwrap_or(0);
        }
        // ANTI-VACUITY: a run where nobody searched would satisfy every equality
        // above with a line of zeros.
        assert!(scored_anywhere > 0, "no seat scored a candidate: {rendered}");
    }

    /// The `adjudicate` report on a real near-terminal root: the label first,
    /// one row per candidate, K replications.
    #[test]
    fn the_adjudicate_command_reports_every_candidate_under_the_label() {
        let rules = Rules::standard();
        let anchor = TempRoot::make("adjudicate");
        let record = rung_record(&rules, world(5));
        std::fs::write(
            anchor.0.join("fixture.json"),
            serde_json::to_string(&record).expect("the record serializes"),
        )
        .expect("the fixture is writable");
        let index = record.events.len() - 2;
        let entries = [entry("tail", "fixture.json", index, SESSION_BOT_SEAT, None)];

        let out = run_with(
            &args(&[
                "adjudicate",
                "tail",
                "--master",
                "0x0B01_2026_0818_0002",
                "--replications",
                "2",
                "--seats",
                "family-weighted,family-weighted",
            ]),
            &entries,
            &anchor.0,
        )
        .expect("the tail adjudication completes");

        assert_eq!(
            out.lines().next(),
            Some(driven::ROBUSTNESS_READ),
            "the first line is not the robustness read",
        );
        assert!(out.contains("all candidates equal"), "no equality line: {out}");
        assert!(out.contains("adjudication-v1"), "no reproduction digest: {out}");
        assert!(out.contains("replications=2"), "K is not echoed: {out}");
    }

    /// The DEFAULTS are the shipped seats, and a driven run through them is a
    /// real run — pinned at a root ONE decision from the end, so the default
    /// (uncapped lookahead on every seat) is affordable in a debug cell.
    #[test]
    fn the_default_seats_drive_a_real_run() {
        let rules = Rules::standard();
        let anchor = TempRoot::make("default-seats");
        let record = rung_record(&rules, world(6));
        std::fs::write(
            anchor.0.join("fixture.json"),
            serde_json::to_string(&record).expect("the record serializes"),
        )
        .expect("the fixture is writable");
        let index = record.events.len() - 1;
        let entries = [entry("last", "fixture.json", index, SESSION_BOT_SEAT, None)];

        let out = run_with(
            &args(&["run", "last", "--master", "0x0B01_2026_0818_0003"]),
            &entries,
            &anchor.0,
        )
        .expect("the last-decision run completes");
        assert!(
            out.contains(&Contestant::LOOKAHEAD_V1_DEFAULT.echo()),
            "the default seat is not the shipped lookahead-v1: {out}",
        );
    }

    /// A CONSTRUCTED source drives too — the runner is not the only entry point
    /// and the module's own surface stays usable without argv.
    #[test]
    fn the_library_surface_stays_usable_without_argv() {
        let rules = Rules::standard();
        let record = rung_record(&rules, world(7));
        let source = RootSource::RecordPrefix { record, event_index: 2 };
        let (state, _) = source.resolve(&rules).expect("the prefix resolves");
        let seats = vec![SeatSpec::Reply(ReplyPolicy::FamilyWeighted); state.players.len()];
        let outcome = driven::drive_rooted(
            &source,
            &seats,
            &rules,
            world(8),
            DrivenOpts::default(),
        )
        .expect("the rooted run completes");
        let rendered = render_outcome(&outcome, "constructed");
        assert!(rendered.contains(&outcome.digest()), "the render drops the digest: {rendered}");
    }

    // =======================================================================
    // Phase C, T6 — the GENERATED root spec (`gen:<predicate>:<master>:<ordinal>`)
    // =======================================================================

    /// The master the two SCANNING cells below run at. Diagnostic by the
    /// constructor's own name, per `game::seeds` — a fixture cell wants
    /// reproduction, which is the case spec §3 carves out.
    const GEN_MASTER: u64 = 0x0C06_2026_0818_9994;

    /// The master the OPENING cells run at, same waiver.
    const OPENING_MASTER: u64 = 0x0C06_2026_0818_9993;

    /// Every `RootPredicate` name parses as a `gen:` root, at either master
    /// spelling, and the parse is a ROUND TRIP: what came back names the
    /// predicate that was typed.
    #[test]
    fn a_gen_root_parses_every_predicate_name_at_either_master_spelling() {
        for predicate in RootPredicate::ALL {
            let spec = format!("gen:{}:0x0A3E_0000_0000_0001:7", predicate.as_str());
            assert_eq!(
                parse_root_ref(&spec).unwrap_or_else(|e| panic!("{spec} must parse: {e}")),
                RootRef::Generated { predicate, master: 0x0A3E_0000_0000_0001, ordinal: 7 },
            );
        }
        // A decimal master and ordinal 0 — the other end of both fields.
        assert_eq!(
            parse_root_ref("gen:final-round-already-triggered:12345:0").expect("decimal parses"),
            RootRef::Generated {
                predicate: RootPredicate::FinalRoundAlreadyTriggered,
                master: 12_345,
                ordinal: 0,
            },
        );
    }

    /// An unknown predicate refuses naming the WHOLE valid set — a caller who
    /// mistyped one gets the vocabulary, not a shrug.
    #[test]
    fn a_gen_root_with_an_unknown_predicate_names_every_valid_name() {
        let err = parse_root_ref("gen:final-round:0x1:0").expect_err("an unknown predicate refuses");
        assert!(err.contains("final-round"), "the refusal hides what was typed: {err}");
        for predicate in RootPredicate::ALL {
            assert!(
                err.contains(predicate.as_str()),
                "the refusal does not name `{}`: {err}",
                predicate.as_str(),
            );
        }
        assert!(err.contains(USAGE), "the refusal drops USAGE: {err}");
    }

    /// Each MALFORMED field refuses on its own terms — arity, master, ordinal,
    /// empty name — and every refusal carries USAGE.
    #[test]
    fn a_malformed_gen_root_refuses_naming_the_field_that_is_wrong() {
        let cases: [(&str, &str); 5] = [
            // Too few fields.
            ("gen:final-round-already-triggered:0x1", "gen:<predicate>"),
            // Too many.
            ("gen:final-round-already-triggered:0x1:0:9", "gen:<predicate>"),
            // A master that is not a number.
            ("gen:final-round-already-triggered:0xzz:0", "0xzz"),
            // An ordinal that is not a decimal u32.
            ("gen:final-round-already-triggered:0x1:-1", "-1"),
            // An empty predicate name.
            ("gen::0x1:0", "\"\""),
        ];
        for (spec, needle) in cases {
            let err = parse_root_ref(spec).unwrap_err();
            assert!(err.contains(needle), "{spec}: the refusal hides {needle}: {err}");
            assert!(err.contains(USAGE), "{spec}: the refusal drops USAGE: {err}");
        }
    }

    /// The `gen:` form reaches EVERY subcommand that takes a root — the
    /// separator and the adjudicator become runnable on generated roots, which
    /// is what the grammar is for.
    #[test]
    fn every_root_taking_subcommand_accepts_a_gen_root() {
        let spec = "gen:achievement-claim-available:0x0A3E_0000_0000_0002:3";
        let expected = RootRef::Generated {
            predicate: RootPredicate::AchievementClaimAvailable,
            master: 0x0A3E_0000_0000_0002,
            ordinal: 3,
        };
        match parse(&args(&["table", spec, "--decision-seed", "9"])).expect("table parses") {
            Command::Table { root, .. } => assert_eq!(root, expected),
            other => panic!("table parsed as {other:?}"),
        }
        match parse(&args(&["run", spec, "--master", "0x1"])).expect("run parses") {
            Command::Run { root, .. } => assert_eq!(root, expected),
            other => panic!("run parsed as {other:?}"),
        }
        match parse(&args(&["adjudicate", spec, "--master", "0x1"])).expect("adjudicate parses") {
            Command::Adjudicate { root, .. } => assert_eq!(root, expected),
            other => panic!("adjudicate parsed as {other:?}"),
        }
    }

    /// A `gen:` root carries no recorded seed, so `table` REFUSES without
    /// `--decision-seed` — the `path:event` sentence, widened to name both
    /// seedless forms rather than leaving one of them unexplained.
    #[test]
    fn a_gen_root_requires_a_decision_seed_at_table() {
        let err = run_with(
            &args(&["table", "gen:final-round-already-triggered:0x1:0"]),
            &[],
            Path::new("."),
        )
        .expect_err("a gen root without a seed must refuse");
        assert!(err.contains("--decision-seed"), "the refusal hides the flag: {err}");
        assert!(err.contains("gen:"), "the refusal does not name the gen form: {err}");
    }

    /// END TO END on the CHEAP family-1 ecology: a `gen:` root RESOLVES, and the
    /// report's header carries the generated provenance the way a registry root
    /// carries its identity — predicate, master, ordinal, the scan's game index,
    /// and the whole `GeneratingEcology`, which is the half that makes a
    /// `(predicate, master, ordinal)` triple an identity at all.
    ///
    /// The rung ecology is deliberate (design §6's wall discipline): the layer-2
    /// predicates scan the v2 LOOKAHEAD ecology, which is minutes.
    #[test]
    fn a_generated_root_resolves_and_prints_its_provenance_in_the_header() {
        let spec = format!("gen:final-round-already-triggered:{GEN_MASTER:#x}:0");
        let out = run_with(
            &args(&["table", &spec, "--decision-seed", "0x0DEC_0000_0000_0001"]),
            &[],
            Path::new("."),
        )
        .expect("the generated root resolves and tables");

        let ecology = driven::GeneratingEcology::v1();
        assert!(out.contains(RootPredicate::FinalRoundAlreadyTriggered.as_str()));
        for needle in [
            format!("{GEN_MASTER:#018x}"),
            "ordinal=0".to_string(),
            "game=".to_string(),
            ecology.id.clone(),
            ecology.driver.clone(),
            ecology.seed_derivation.clone(),
            format!("per_game_cap={}", ecology.per_game_cap),
            format!("games={}", ecology.games),
        ] {
            assert!(out.contains(&needle), "the header drops `{needle}`:\n{out}");
        }
        // A generated root has no recorded move, so the reproduction line is the
        // NOT CHECKED arm rather than a verdict about a move nobody recorded.
        assert!(out.contains("NOT CHECKED"), "a generated root claims a reproduction:\n{out}");
    }

    // =======================================================================
    // Phase C, T6 — the seat grammar's fourth menu slot
    // =======================================================================

    /// `race-rusher` parses as the fourth REPLY-MENU slot — argv-REACHABLE, so
    /// T7's pilots can name it in `--seats` — and every menu name round-trips
    /// through its own `as_str`.
    #[test]
    fn the_seat_grammar_reads_race_rusher_as_the_fourth_menu_slot() {
        assert_eq!(
            parse_seat("race-rusher").expect("race-rusher is a seat"),
            SeatSpec::Reply(ReplyPolicy::RaceRusher),
        );
        // HAND-MAINTAINED on both sides (the literal array in `parse_seat` and
        // the USAGE list), so the pin names all four rather than sweeping a
        // const that does not exist.
        for policy in [
            ReplyPolicy::FamilyWeighted,
            ReplyPolicy::GreedyAtFinal,
            ReplyPolicy::LookaheadAtFinal,
            ReplyPolicy::RaceRusher,
        ] {
            assert_eq!(
                parse_seat(policy.as_str()).unwrap_or_else(|e| panic!("{policy:?}: {e}")),
                SeatSpec::Reply(policy),
                "{policy:?} does not round-trip through the seat grammar",
            );
            assert!(
                USAGE.contains(policy.as_str()),
                "USAGE does not name the seat `{}`",
                policy.as_str(),
            );
        }
        // In a LIST, beside a contestant — the shape `--seats` actually takes.
        assert_eq!(
            parse_seats("race-rusher, greedy-v1").expect("a mixed seat list parses"),
            vec![
                SeatSpec::Reply(ReplyPolicy::RaceRusher),
                SeatSpec::Contestant(Contestant::GREEDY_V1_DEFAULT),
            ],
        );
    }

    // =======================================================================
    // Phase C, T6 — `family <id> --block <kind> [--arm <arm>] [--master <u64>]`
    // =======================================================================

    /// The verb parses: every declared family id, both blocks, both arms, and
    /// the master when one is typed.
    #[test]
    fn the_family_command_parses_every_id_block_and_arm() {
        for decl in families() {
            match parse(&args(&["family", decl.id, "--block", "pilot", "--master", "0x11"]))
                .unwrap_or_else(|e| panic!("{}: {e}", decl.id))
            {
                Command::Family { id, block, master, arm } => {
                    assert_eq!(id, decl.id);
                    assert_eq!(block, BlockKind::Pilot);
                    assert_eq!(master, Some(0x11));
                    assert_eq!(arm, FamilyArm::Positive, "the default arm is not positive");
                }
                other => panic!("{}: parsed as {other:?}", decl.id),
            }
        }
        match parse(&args(&[
            "family",
            "exclusive-race",
            "--block",
            "confirmatory",
            "--arm",
            "control",
        ]))
        .expect("a confirmatory control arm parses")
        {
            Command::Family { id, block, master, arm } => {
                assert_eq!(id, "exclusive-race");
                assert_eq!(block, BlockKind::Confirmatory);
                assert_eq!(master, None);
                assert_eq!(arm, FamilyArm::Control);
            }
            other => panic!("parsed as {other:?}"),
        }
    }

    /// An unknown family id refuses naming ALL FOUR declared ids.
    #[test]
    fn an_unknown_family_id_names_the_four_declared_ids() {
        let err = parse(&args(&["family", "final-rond", "--block", "pilot", "--master", "1"]))
            .expect_err("an unknown family must refuse");
        assert!(err.contains("final-rond"), "the refusal hides what was typed: {err}");
        for decl in families() {
            assert!(err.contains(decl.id), "the refusal does not name `{}`: {err}", decl.id);
        }
        assert!(err.contains(USAGE), "the refusal drops USAGE: {err}");
    }

    /// An unknown block name refuses naming THE TWO valid blocks — and the
    /// ESCROW-ABSENT property rides this arm: no grammar names an escrow block,
    /// so `--block escrow` is exactly as unknown as `--block wibble` and the
    /// refusal does not offer it as an alternative (design §2's escrow hygiene:
    /// escrow ordinals live in the freeze DOCUMENT so no command can burn one).
    #[test]
    fn an_unknown_block_name_refuses_naming_the_two_valid_blocks_and_no_third() {
        for typed in ["escrow", "wibble"] {
            let err = parse(&args(&["family", "final-round", "--block", typed])).unwrap_err();
            assert!(err.contains(typed), "the refusal hides what was typed: {err}");
            assert!(err.contains("pilot"), "the refusal does not name `pilot`: {err}");
            assert!(
                err.contains("confirmatory"),
                "the refusal does not name `confirmatory`: {err}",
            );
        }
        assert_eq!(BLOCK_NAMES, ["pilot", "confirmatory"], "the block vocabulary moved");
        assert!(!USAGE.contains("escrow block"), "USAGE names an escrow block");
    }

    /// `--block` is REQUIRED and an unknown `--arm` refuses naming both arms.
    #[test]
    fn the_family_command_requires_a_block_and_names_both_arms() {
        let no_block = parse(&args(&["family", "final-round", "--master", "1"]))
            .expect_err("a family run with no block must refuse");
        assert!(no_block.contains("--block"), "the refusal hides the flag: {no_block}");

        let bad_arm =
            parse(&args(&["family", "final-round", "--block", "pilot", "--arm", "sideways"]))
                .expect_err("an unknown arm must refuse");
        assert!(bad_arm.contains("sideways"), "the refusal hides what was typed: {bad_arm}");
        for arm in ARM_NAMES {
            assert!(bad_arm.contains(arm), "the refusal does not name `{arm}`: {bad_arm}");
        }
    }

    /// The two MASTER refusals, end to end and COSTING NOTHING — both fire in
    /// `resolve_block` before a single draw is resolved.
    ///
    /// Pilot without `--master`: recorded-before-use stays the caller's
    /// obligation. Confirmatory WITH one: the declaration's master is the
    /// committed identity, and a typo must not be able to run a
    /// plausible-looking non-frozen block.
    #[test]
    fn a_pilot_needs_a_master_and_a_confirmatory_refuses_one() {
        let missing =
            run_with(&args(&["family", "final-round", "--block", "pilot"]), &[], Path::new("."))
                .expect_err("a pilot with no master must refuse");
        assert!(missing.contains("recorded-before-use"), "the refusal loses its reason: {missing}");
        assert!(missing.contains("final-round"), "the refusal does not name the family: {missing}");
        assert!(missing.contains(USAGE), "the command-shaped refusal drops USAGE: {missing}");

        let supplied = run_with(
            &args(&["family", "final-round", "--block", "confirmatory", "--master", "0x1"]),
            &[],
            Path::new("."),
        )
        .expect_err("a confirmatory with a caller's master must refuse");
        assert!(
            supplied.contains("caller-supplied master is refused"),
            "the refusal loses its sentence: {supplied}",
        );
        assert!(supplied.contains(USAGE), "the command-shaped refusal drops USAGE: {supplied}");
    }

    /// BEFORE THE FREEZE every family's confirmatory block refuses BY NAME —
    /// the pre-freeze state, pinned, so the freeze commit is what makes these
    /// runnable and nothing else can.
    #[test]
    fn every_confirmatory_block_refuses_by_name_before_the_freeze() {
        for decl in families() {
            assert!(
                decl.confirmatory.is_none(),
                "{} carries a frozen block already — this pin is the PRE-FREEZE state",
                decl.id,
            );
            let err =
                run_with(&args(&["family", decl.id, "--block", "confirmatory"]), &[], Path::new("."))
                    .expect_err("a confirmatory block must refuse before the freeze");
            assert!(
                err.contains(decl.id) && err.contains("FREEZE"),
                "{}: the refusal does not name the family and the freeze: {err}",
                decl.id,
            );
        }
    }

    // =======================================================================
    // Phase C, T6 — the `--arm control` mapping
    // =======================================================================

    /// `--arm control` re-points each family at its DECLARED control (or corner)
    /// generator, and — where the control asks a different question — at the
    /// control PASS RULE the declaration's own docs name.
    ///
    /// The mapping, as landed:
    ///
    /// | family           | control generator                | control rule                           |
    /// |------------------|----------------------------------|----------------------------------------|
    /// | `final-round`    | `TriggerRoundNotFinal` predicate | unchanged (`AdjudicatedBest`)          |
    /// | `end-trigger`    | `FAMILY2_CONTROL_GENERATOR`      | `Turn(NotEndFired)`                    |
    /// | `exclusive-race` | `RACE_CONTROL_GENERATOR`         | `Turn(DenialNotTaken)`                 |
    /// | `achievement`    | `PROXIMITY_CORNER_GENERATOR`     | unchanged (`Turn(AchievementClaimed)`) |
    #[test]
    fn the_control_arm_repoints_each_family_at_its_declared_arm_and_rule() {
        let by_id = |id: &str| {
            families().into_iter().find(|d| d.id == id).unwrap_or_else(|| panic!("{id} declared"))
        };

        let family1 = control_arm(&by_id("final-round")).expect("family 1 has a control");
        assert_eq!(
            family1.generator,
            FamilyGenerator::Predicate(RootPredicate::TriggerRoundNotFinal),
        );
        assert_eq!(family1.rule, by_id("final-round").rule, "family 1's rule moved");

        let family2 = control_arm(&by_id("end-trigger")).expect("family 2 has a control");
        assert_eq!(family2.generator, FAMILY2_CONTROL_GENERATOR);
        assert_eq!(family2.rule, PassRule::Turn(TurnPredicate::NotEndFired));

        let family3 = control_arm(&by_id("exclusive-race")).expect("family 3 has a control");
        assert_eq!(family3.generator, RACE_CONTROL_GENERATOR);
        assert_eq!(
            family3.rule,
            PassRule::Turn(TurnPredicate::DenialNotTaken(DECLARED_BY_THE_ROOT)),
            "family 3's control does not read the DENIAL rule",
        );
        assert!(family3.rule.names_a_step(), "the control rule must still be re-pointed per root");

        let family4 = control_arm(&by_id("achievement")).expect("family 4 has a corner");
        assert_eq!(family4.generator, PROXIMITY_CORNER_GENERATOR);
        assert_eq!(family4.rule, by_id("achievement").rule, "family 4's rule moved");

        // Every other field is the declaration's, untouched: an arm is a
        // GENERATOR and a RULE, never a different family.
        for (control, id) in [
            (&family1, "final-round"),
            (&family2, "end-trigger"),
            (&family3, "exclusive-race"),
            (&family4, "achievement"),
        ] {
            let positive = by_id(id);
            assert_eq!(control.id, positive.id);
            assert_eq!(control.label, positive.label);
            assert_eq!(control.class, positive.class);
            assert_eq!(control.replications, positive.replications);
            assert_eq!(control.per_root_f, positive.per_root_f);
            assert_eq!(control.menu, positive.menu);
            assert_eq!(control.pilot, positive.pilot);
            assert_ne!(
                control.generator, positive.generator,
                "{id}: a control equal to its positive",
            );
        }
    }

    /// An id no declaration carries refuses rather than silently returning the
    /// positive arm — the defensive half of an exhaustive mapping.
    #[test]
    fn the_control_arm_refuses_a_family_it_does_not_know() {
        let mut invented = families().remove(0);
        invented.id = "not-a-family";
        let err = control_arm(&invented).expect_err("an unknown id must refuse");
        assert!(err.contains("not-a-family"), "the refusal hides the id: {err}");
        for decl in families() {
            assert!(err.contains(decl.id), "the refusal does not name `{}`: {err}", decl.id);
        }
    }

    // =======================================================================
    // Phase C, T6 — rendering a FamilyBlockReport (CONSTRUCTED outcomes only)
    // =======================================================================

    fn counter_check(tied: usize, band: usize, distinct: usize) -> CounterCheck {
        CounterCheck {
            table: table_with(
                Move::SkipBuilding,
                &[(Move::SkipBuilding, 12), (Move::BuildTradingHouse, 11)],
            ),
            tied_at_top: tied,
            top_band: band,
            distinct_values: distinct,
        }
    }

    fn turn_replication(r: u32, verdict: bool) -> ReplicationRead {
        ReplicationRead {
            replication: r,
            decision_seed: 0x0D00_0000_0000_0000 + u64::from(r),
            root_word: 0x00FF_0000_0000_00AB,
            verdict: Some(verdict),
            pick: None,
            turn_moves: 4,
            handoff_digest: Some(format!("handoff-{r}")),
            game_over: Some(false),
            counters: SeatCounters::NONE,
        }
    }

    fn turn_read(misses: u32, r: u32) -> FamilyReadOutcome {
        FamilyReadOutcome {
            rule: PassRule::Turn(TurnPredicate::EndFired),
            grain: ReadGrain::Turn,
            master: 0x0A3E_0000_0000_0001,
            root_word: 0x00FF_0000_0000_00AB,
            root_digest: "00ff0000000000ab".to_string(),
            subject_echo: Contestant::LOOKAHEAD_V1_DEFAULT.echo(),
            replications: (0..r).map(|i| turn_replication(i, i >= misses)).collect(),
            counter_check: counter_check(2, 2, 2),
            annotations: vec![CounterAnnotation::TiedAtTop { candidates: 2 }],
        }
    }

    fn policy_admission(policy: ReplyPolicy, verdict: AdmissionVerdict) -> PolicyAdmission {
        PolicyAdmission {
            policy,
            true_n: 4,
            false_n: 4,
            true_mean: 12.0,
            false_mean: 1.0,
            difference: 11.0,
            verdict,
        }
    }

    fn family_row(ordinal: u32, verdict: RootVerdict, read: Option<FamilyReadOutcome>) -> RootRow {
        RootRow {
            ordinal,
            descriptor: descriptor(),
            draw_seed: Some(0x5EED_0000_0000_0000 + u64::from(ordinal)),
            step: None,
            rule: PassRule::Turn(TurnPredicate::EndFired),
            admission: Some(AdmissionRead {
                delta: 5,
                control: false,
                policies: vec![
                    policy_admission(ReplyPolicy::FamilyWeighted, AdmissionVerdict::Dominates),
                    policy_admission(ReplyPolicy::GreedyAtFinal, AdmissionVerdict::Dominates),
                ],
            }),
            dead_by_policy: Vec::new(),
            dead: false,
            top_sets: Vec::new(),
            read,
            misses: 0,
            unresolved: 0,
            verdict,
            lookahead_sole_dissenter: false,
            rider: None,
        }
    }

    fn family_report(verdict: FamilyVerdict, roots: Vec<RootRow>) -> FamilyBlockReport {
        let drawn = roots.len();
        let admitted = roots.iter().filter(|r| r.read.is_some()).count();
        let counter_refused =
            roots.iter().filter(|r| matches!(r.verdict, RootVerdict::CounterRefused(_))).count();
        let sole_dissenters =
            roots.iter().filter(|r| r.lookahead_sole_dissenter).map(|r| r.ordinal).collect();
        FamilyBlockReport {
            id: "end-trigger",
            label: FamilyLabel::Confirmatory,
            class: StrategicClass::EndTimingProximity,
            block: BlockKind::Pilot,
            generator: "end-firable-ahead-in-truth",
            master: 0x0A3E_0000_0000_0001,
            subject_echo: Contestant::LOOKAHEAD_V1_DEFAULT.echo(),
            rule: PassRule::Turn(TurnPredicate::EndFired),
            replications: 20,
            per_root_f: 1,
            menu: vec![ReplyPolicy::FamilyWeighted, ReplyPolicy::GreedyAtFinal],
            admission_k: 4,
            delta: 5,
            control: false,
            roots,
            drawn,
            admitted,
            counter_refused,
            sole_dissenters,
            verdict,
            robustness_read: driven::ROBUSTNESS_READ,
        }
    }

    fn line_index(rendered: &str, needle: &str) -> usize {
        rendered
            .lines()
            .position(|line| line.contains(needle))
            .unwrap_or_else(|| panic!("the report has no line containing {needle:?}:\n{rendered}"))
    }

    /// **THE SECTION ORDER IS THE PIN** (design §2): the family report leads
    /// with its LABEL — what a pass would even claim — and D7's STEP-ONE
    /// counter check comes second, before any per-policy arithmetic. A report
    /// that put the numbers first would invite a reader to quote them before
    /// learning whether the instrument refused the framing.
    #[test]
    fn the_family_report_leads_with_the_label_then_the_counter_check() {
        let report = family_report(
            FamilyVerdict::Pass,
            vec![family_row(0, RootVerdict::Pass, Some(turn_read(0, 4)))],
        );
        let rendered = render_family_block(&report, FamilyArm::Positive);

        let label = line_index(&rendered, "CONFIRMATORY");
        let counter = line_index(&rendered, "counter check");
        let admission = line_index(&rendered, "per-policy admission");
        assert!(rendered.starts_with("=== driven family ===\n"), "no banner:\n{rendered}");
        assert_eq!(label, 1, "the LABEL is not the first content line:\n{rendered}");
        assert!(label < counter, "the counter check precedes the label:\n{rendered}");
        assert!(counter < admission, "the admission table precedes the counter check:\n{rendered}");
        // The identity a reader needs beside the label.
        for needle in ["end-trigger", "pilot", "arm=positive", "end-firable-ahead-in-truth"] {
            assert!(rendered.contains(needle), "the header drops `{needle}`:\n{rendered}");
        }
        assert!(
            rendered.contains(driven::ROBUSTNESS_READ),
            "the family report drops the robustness read:\n{rendered}",
        );
    }

    /// EVERY replication prints its own DECISION SEED and the root word
    /// verbatim — T7's separator feeds a failing replication's seed straight to
    /// `table … --decision-seed`, and a seed a reader must re-derive from a mix
    /// formula is a seed nobody feeds anywhere.
    #[test]
    fn the_family_report_prints_every_replication_seed_and_root_word() {
        let read = turn_read(1, 3);
        let report = family_report(
            FamilyVerdict::Pass,
            vec![family_row(0, RootVerdict::Pass, Some(read.clone()))],
        );
        let rendered = render_family_block(&report, FamilyArm::Positive);
        for replication in &read.replications {
            assert!(
                rendered.contains(&format!("{:#018x}", replication.decision_seed)),
                "replication {}'s decision seed is missing:\n{rendered}",
                replication.replication,
            );
        }
        assert!(
            rendered.contains(&format!("{:#018x}", read.root_word)),
            "the root word is missing:\n{rendered}",
        );
        // The DRAW SEED where one is recorded — a builder draw's only
        // reproduction handle.
        assert!(
            rendered.contains(&format!("{:#018x}", 0x5EED_0000_0000_0000_u64)),
            "the draw seed is missing:\n{rendered}",
        );
        // The exact binomial line, verbatim from the outcome that computed it.
        assert!(
            rendered.contains(&read.binomial_line().expect("a turn-grain read has one")),
            "the binomial line is missing:\n{rendered}",
        );
    }

    /// A PREDICATE draw records no draw seed, and the render says so rather
    /// than inventing one — `None` is the honest shape for a scanned root
    /// (`RootRow::draw_seed`'s own rule).
    #[test]
    fn a_predicate_draws_absent_seed_renders_as_absent() {
        let mut row = family_row(0, RootVerdict::Pass, Some(turn_read(0, 2)));
        row.draw_seed = None;
        let rendered = render_family_block(
            &family_report(FamilyVerdict::Pass, vec![row]),
            FamilyArm::Positive,
        );
        assert!(
            rendered.contains("draw seed      -"),
            "an absent draw seed does not render as absent:\n{rendered}",
        );
    }

    /// The per-policy ADMISSION table, the EXCLUSION block and the DISSENT
    /// block — the three things a reader needs to tell "the subject failed"
    /// from "the root was never in the denominator".
    #[test]
    fn the_family_report_renders_admission_exclusions_and_the_dissent_block() {
        let mut excluded =
            family_row(1, RootVerdict::Excluded(ExclusionReason::NotDemonstrated), None);
        excluded.admission = Some(AdmissionRead {
            delta: 5,
            control: false,
            policies: vec![
                policy_admission(ReplyPolicy::FamilyWeighted, AdmissionVerdict::Dominates),
                policy_admission(ReplyPolicy::LookaheadAtFinal, AdmissionVerdict::Fails),
            ],
        });
        excluded.lookahead_sole_dissenter = true;
        let dead = family_row(2, RootVerdict::Excluded(ExclusionReason::Dead), None);

        let report = family_report(
            FamilyVerdict::Pass,
            vec![family_row(0, RootVerdict::Pass, Some(turn_read(0, 2))), excluded, dead],
        );
        let rendered = render_family_block(&report, FamilyArm::Positive);

        // The table: one row per (root, policy), with the arithmetic δ is
        // compared to.
        assert!(rendered.contains("per-policy admission"), "no admission table:\n{rendered}");
        assert!(rendered.contains("DOMINATES"), "no dominance verdict:\n{rendered}");
        assert!(rendered.contains("FAILS"), "no failing verdict:\n{rendered}");
        assert!(rendered.contains("11.00"), "the difference is missing:\n{rendered}");

        // The exclusions, with their typed reasons.
        assert!(rendered.contains("exclusions"), "no exclusion block:\n{rendered}");
        assert!(rendered.contains("NOT DEMONSTRATED"), "no not-demonstrated reason:\n{rendered}");
        assert!(rendered.contains("DEAD"), "no dead reason:\n{rendered}");

        // The dissent block, and the circularity sentence it exists for.
        assert!(
            rendered.contains("SOLE DISSENTER"),
            "the sole-dissenter routing is missing:\n{rendered}",
        );
        assert!(
            rendered.to_lowercase().contains("suppression"),
            "the sole-dissenter block loses its reason:\n{rendered}",
        );
    }

    /// A COUNTER-REFUSED root renders as a ROUTING and never as a failure —
    /// D7's branch 0, which is the one reading a family report must not
    /// mis-summarize.
    #[test]
    fn a_counter_refused_root_renders_as_a_routing_not_a_failure() {
        let refused = family_row(
            0,
            RootVerdict::CounterRefused("ply_cap_hits=3 — a truncated search's pick".to_string()),
            None,
        );
        let rendered = render_family_block(
            &family_report(FamilyVerdict::Pass, vec![refused]),
            FamilyArm::Positive,
        );
        assert!(rendered.contains("COUNTER REFUSED"), "no counter-refusal row:\n{rendered}");
        assert!(rendered.contains("ply_cap_hits=3"), "the evidence is dropped:\n{rendered}");
        assert!(
            rendered.to_lowercase().contains("routing"),
            "a refusal reads as a failure:\n{rendered}",
        );
        assert!(rendered.contains("counter-refused=1"), "the count is missing:\n{rendered}");
    }

    /// The VOID and EXISTENCE verdicts render DISTINCTLY — a block that
    /// confirms nothing must not be readable as a quiet pass, and an existence
    /// demonstration must not be readable as a GO.
    #[test]
    fn the_void_and_existence_verdicts_render_distinctly() {
        let void = family_report(
            FamilyVerdict::VoidAsConfirmation {
                admitted: 2,
                drawn: 6,
                minimum: MIN_ADMITTED_ROOTS,
                disposition: VOID_DISPOSITION,
            },
            vec![family_row(0, RootVerdict::Pass, Some(turn_read(0, 2)))],
        );
        let rendered = render_family_block(&void, FamilyArm::Positive);
        assert!(rendered.contains("VOID AS CONFIRMATION"), "the void verdict is quiet:\n{rendered}");
        assert!(
            rendered.contains("CONFIRMS NOTHING"),
            "the void verdict does not say what it means:\n{rendered}",
        );
        assert!(rendered.contains(VOID_DISPOSITION), "the disposition is dropped:\n{rendered}");

        let mut existence = family_report(
            FamilyVerdict::GapDemonstrated { roots: 3 },
            vec![family_row(0, RootVerdict::GapRecorded { misses: 2 }, Some(turn_read(2, 4)))],
        );
        existence.label = FamilyLabel::ExistenceDemonstration;
        existence.id = "exclusive-race";
        let rendered = render_family_block(&existence, FamilyArm::Positive);
        assert!(
            rendered.contains("EXISTENCE_DEMONSTRATION"),
            "the existence label is missing:\n{rendered}",
        );
        assert!(rendered.contains("GAP DEMONSTRATED"), "the gap verdict is missing:\n{rendered}");
        assert!(
            rendered.contains("never a GO"),
            "the existence arm does not say what it cannot support:\n{rendered}",
        );
        assert!(rendered.contains("GAP RECORDED"), "the per-root gap row is missing:\n{rendered}");
    }

    /// Rendering is a pure function of the report plus the arm: the same pair
    /// renders byte-identically twice, and the ARM reaches the page — including
    /// the DOUBLE-NEGATION sentence, which is the one thing a reader of a
    /// control block could otherwise get backwards.
    #[test]
    fn the_family_render_is_deterministic_and_the_control_arm_states_its_direction() {
        let report = family_report(
            FamilyVerdict::Pass,
            vec![family_row(0, RootVerdict::Pass, Some(turn_read(0, 3)))],
        );
        let control = render_family_block(&report, FamilyArm::Control);
        assert_eq!(control, render_family_block(&report, FamilyArm::Control));
        assert_ne!(
            control,
            render_family_block(&report, FamilyArm::Positive),
            "the arm does not reach the report",
        );
        assert!(control.contains("arm=control"), "the arm is not named:\n{control}");
        assert!(
            control.contains("double negation"),
            "the control arm does not state its admission direction:\n{control}",
        );
    }

    // =======================================================================
    // Phase C, T6 — the `opening` probe
    // =======================================================================

    /// The verb parses, and its two required flags refuse when absent.
    #[test]
    fn the_opening_command_parses_and_requires_its_master_and_games() {
        match parse(&args(&["opening", "--master", "0x2A", "--games", "3"])).expect("parses") {
            Command::Opening { master, games, contestant } => {
                assert_eq!(master, 0x2A);
                assert_eq!(games, 3);
                assert_eq!(contestant, Contestant::LOOKAHEAD_V1_DEFAULT);
            }
            other => panic!("parsed as {other:?}"),
        }
        match parse(&args(&[
            "opening",
            "--master",
            "1",
            "--games",
            "2",
            "--contestant",
            "greedy-v1",
        ]))
        .expect("a subject spec parses")
        {
            Command::Opening { contestant, .. } => {
                assert_eq!(contestant, Contestant::GREEDY_V1_DEFAULT);
            }
            other => panic!("parsed as {other:?}"),
        }

        let no_master = parse(&args(&["opening", "--games", "2"])).expect_err("no master refuses");
        assert!(no_master.contains("--master"), "{no_master}");
        let no_games = parse(&args(&["opening", "--master", "1"])).expect_err("no games refuses");
        assert!(no_games.contains("--games"), "{no_games}");
        let zero = parse(&args(&["opening", "--master", "1", "--games", "0"]))
            .expect_err("zero games refuses");
        assert!(zero.contains("--games"), "{zero}");
        let positional = parse(&args(&["opening", "3", "--master", "1", "--games", "2"]))
            .expect_err("a stray positional refuses");
        assert!(positional.contains("positional"), "{positional}");
    }

    /// **THE SETUP-EQUIVALENCE PIN** (plan review SHOULD-5). The probe builds
    /// its opening through the PUBLIC path — `new_game`, each seat's own
    /// `ArenaAgent::choose_starting_order` in seat order, `finalize_setup` —
    /// because the Phase-B split seamed out only `play_from_state` and the
    /// arena's setup half is still interior to its private driver. The pin is
    /// that the two agree BIT FOR BIT at the same master and the same streams:
    /// the built opening equals `state_at(record, 0)` of a `run_arena_game`
    /// there.
    ///
    /// Rung seats, one cell — the property is STRUCTURAL (which stream feeds
    /// which Keep), never a fact about the subject.
    #[test]
    fn the_opening_setup_path_is_bit_identical_to_the_arena_setup_half() {
        let rules = Rules::standard();
        let subject = Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT);
        let probe = run_opening(OPENING_MASTER, 2, subject, &rules).expect("the probe runs");
        assert_eq!(probe.rows.len(), 2, "the probe played the wrong number of games");

        for row in &probe.rows {
            let world = WorldSeed(row.world);
            let mut make = |seat: PlayerId, _players: u8| -> Box<dyn ArenaAgent> {
                subject.build(&rules, agent_stream_seed(world, seat, Slot(seat.0))).0
            };
            let stats = run_arena_game(world, 2, &rules, &mut make, true)
                .expect("the comparison game completes");
            let arena_opening =
                state_at(&stats.record, 0, &rules).expect("the record's position 0 replays");
            assert_eq!(
                row.opening_digest,
                state_digest(&arena_opening),
                "game {}: the probe's opening is not the arena's",
                row.game_index,
            );
        }
    }

    /// The probe plays the FIRST TURN of the first-moving seat and reads the
    /// three things the C-D question is about off it.
    #[test]
    fn the_opening_probe_reads_placements_president_and_purchase_off_the_first_turn() {
        let rules = Rules::standard();
        let probe = run_opening(
            OPENING_MASTER,
            3,
            Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
            &rules,
        )
        .expect("the probe runs");
        assert_eq!(probe.rows.len(), 3);
        for row in &probe.rows {
            assert!(row.moves > 0, "game {}: an empty first turn", row.game_index);
            assert!(
                !row.game_over,
                "game {}: the first turn ended the game, which no opening does",
                row.game_index,
            );
            // The president is PLACED from hand on turn one and the area action
            // follows it — both are facts about the rules, not about the seat.
            assert!(row.president.is_some(), "game {}: no president area", row.game_index);
            assert_ne!(row.purchase, OPENING_NO_ACTION, "game {}: no area action", row.game_index);
            assert_eq!(
                row.decision_seed,
                agent_stream_seed(WorldSeed(row.world), row.mover, Slot(row.mover.0)),
                "the turn is not taken on the arena's own keying",
            );
        }
        // Distinct worlds per game — the schedule advances.
        let worlds: std::collections::BTreeSet<u64> = probe.rows.iter().map(|r| r.world).collect();
        assert_eq!(worlds.len(), 3, "two games share a world");
    }

    /// The RENDER, on a CONSTRUCTED tally: the distribution, the lab-rush share
    /// with its Wilson interval, and the SETUP-PATH line the ecology discipline
    /// asks for (design NOTE-4).
    #[test]
    fn the_opening_render_carries_the_distribution_the_wilson_interval_and_the_setup_path() {
        let row = |index: u32, president: &str, purchase: &str| OpeningGame {
            game_index: index,
            world: 0x0A00 + u64::from(index),
            mover: PlayerId(0),
            decision_seed: 0x0D00 + u64::from(index),
            opening_digest: format!("opening-{index}"),
            placements: vec!["Bank".to_string(), "CopperMine".to_string()],
            president: Some(president.to_string()),
            purchase: purchase.to_string(),
            moves: 5,
            game_over: false,
        };
        let probe = OpeningProbe {
            master: 0x0A3E_0000_0000_0003,
            games: 4,
            subject_echo: Contestant::LOOKAHEAD_V1_DEFAULT.echo(),
            rows: vec![
                row(0, "LaboratoryA", "Laboratory"),
                row(1, "LaboratoryA", "Laboratory"),
                row(2, "Bank", "Bank"),
                row(3, "PortA", "Port"),
            ],
        };
        let rendered = render_opening(&probe);

        // The distribution, tallied and shared.
        assert!(rendered.contains("Bank+CopperMine"), "no placement key:\n{rendered}");
        assert!(rendered.contains("LaboratoryA"), "no president area:\n{rendered}");
        assert!(rendered.contains("0.5000"), "no share for the 2-of-4 cell:\n{rendered}");

        // The lab-rush share and its interval, through the crate's OWN Wilson
        // helper so this figure means what every other rate in the repo means.
        let wilson = wilson_95(2, 4);
        assert!(rendered.contains("lab rush"), "no lab-rush line:\n{rendered}");
        assert!(
            rendered.contains(&format!("{:.4}", wilson.lo))
                && rendered.contains(&format!("{:.4}", wilson.hi)),
            "the Wilson interval is missing:\n{rendered}",
        );

        // The setup path, printed rather than assumed.
        assert!(
            rendered.contains(OPENING_SETUP_PATH),
            "the setup-path line is missing:\n{rendered}",
        );
        assert!(rendered.contains(&probe.subject_echo), "the subject echo is missing:\n{rendered}");
    }

    /// ONE cell with the SHIPPED subject — the probe's real question is about
    /// `lookahead-v1`, and a machinery-only pin at a rung would leave the
    /// expensive path unexercised. One game, one turn (~3s debug budget).
    #[test]
    fn one_opening_game_runs_the_shipped_lookahead_subject() {
        let rules = Rules::standard();
        let probe = run_opening(OPENING_MASTER, 1, Contestant::LOOKAHEAD_V1_DEFAULT, &rules)
            .expect("the shipped subject plays an opening");
        assert_eq!(probe.rows.len(), 1);
        assert_eq!(probe.subject_echo, Contestant::LOOKAHEAD_V1_DEFAULT.echo());
        assert!(probe.rows[0].moves > 0, "the shipped subject played no move");
        let rendered = render_opening(&probe);
        assert!(rendered.contains(&Contestant::LOOKAHEAD_V1_DEFAULT.echo()), "{rendered}");
    }

    // =======================================================================
    // Phase C, T6 — USAGE and the unchanged surfaces
    // =======================================================================

    /// USAGE documents every NEW verb, flag and vocabulary: the `gen:` root form
    /// and its predicate names, the four family ids, the two blocks, the two
    /// arms, and the two master rules a caller cannot infer from a flag list.
    #[test]
    fn the_usage_documents_the_generated_roots_the_families_and_the_opening_probe() {
        assert!(USAGE.contains("gen:<predicate>:<master>:<ordinal>"), "USAGE has no gen: form");
        for predicate in RootPredicate::ALL {
            assert!(
                USAGE.contains(predicate.as_str()),
                "USAGE does not name the predicate `{}`",
                predicate.as_str(),
            );
        }
        for decl in families() {
            assert!(USAGE.contains(decl.id), "USAGE does not name family `{}`", decl.id);
        }
        for block in BLOCK_NAMES {
            assert!(USAGE.contains(block), "USAGE does not name block `{block}`");
        }
        for arm in ARM_NAMES {
            assert!(USAGE.contains(arm), "USAGE does not name arm `{arm}`");
        }
        assert!(
            USAGE.contains("recorded-before-use"),
            "USAGE does not state the pilot master rule",
        );
        assert!(
            USAGE.contains("REFUSED"),
            "USAGE does not state that a confirmatory block refuses a caller's master",
        );
    }

    /// `list` is UNCHANGED — byte for byte, on a cell-owned registry. The
    /// grammar grew two verbs and a root form; the report a reader already knows
    /// did not move.
    #[test]
    fn the_list_report_is_byte_identical_to_its_golden() {
        let anchor = TempRoot::make("list-golden");
        let entries = [entry("golden", "nothing.json", 3, PlayerId(1), None)];
        let rendered = run_with(&args(&["list"]), &entries, &anchor.0).expect("list always answers");
        let expected = format!(
            "=== driven registry ===\n  \
             1 entries, resolved against {}\n  \
             a MISSING record is a MISS and never a failure: the session roots live in gitignored \
             saves\n\n  \
             golden  [MISSING]\n      \
             path    nothing.json\n      \
             event   3   seat p1   decision seed 0x0dec151000000001\n      \
             move    (none recorded)\n      \
             note    a cell-owned fixture",
            anchor.0.display(),
        );
        assert_eq!(rendered, expected);
    }
}
