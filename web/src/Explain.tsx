// The explanation surface: what the bot would play, and why it played what it
// played.
//
// **This is the one screen a terminal genuinely could not draw.** The CLI shows
// the same numbers as an aligned five-column table, which is the best a
// character grid can do; `eval::explain` is structured, all-`i64` and
// float-free, so a bar chart of it loses nothing and shows the shape of a
// decision at a glance.
//
// THE RULE THE CHART OBEYS, and the reason the payload is built the way it is:
//
//     total === sum(terms[].product) + (terminal ? terminal.value : 0)
//
// The payload carries all 34 features, zero-product ones included — NOT because
// the sum needs them (a zero term contributes zero; an earlier version of this
// comment said otherwise and a reviewer did the arithmetic) but because "8 of
// 34" is only a claim a reader can check if all 34 are there. This component
// COLLAPSES them — a screen of 26 empty rows buries the eight that decided the
// move — and collapsing is not dropping: the total below the fold is the true
// total, the count of what is hidden is printed, and one click brings them back.

import { useState } from "react";
import type {
  Counter,
  Explanation,
  Labels,
  Ranked,
  RefusalRow,
  SuggestResponse,
  WhyResponse,
} from "./types";

/** How many runner-ups to show before the rest go behind a disclosure.
 *
 *  The terminal shows three and cannot show more. A real position offers
 *  ~200 candidates and the server sends all of them, deliberately — showing
 *  more than three is one of the things a browser can do that a terminal
 *  cannot, so the cap lives HERE, where it can be lifted by a click, rather
 *  than on the wire where it would be a silent truncation. */
const RUNNER_UPS_SHOWN = 6;

/** A feature's snake_case token, in words.
 *
 *  The TOKEN is what the wire carries and what every other instrument (the
 *  arena report, `eval_probe`, the CLI table) uses, so it stays the identity;
 *  this is presentation only, and a feature with no entry falls back to the
 *  token rather than to a blank. Both are shown — the words to read, the token
 *  to grep — because a reader comparing this screen with a recorded figure
 *  needs the second one. */
function featureWords(feature: string, labels: Labels | null): string {
  const named = labels?.features?.[feature];
  if (named) return named;
  return feature.replace(/_/g, " ");
}

/** The diverging bar chart.
 *
 *  Sorted by |product| descending, so the terms that decided the move are at
 *  the top and the reader does not have to scan for them. Bars grow from a
 *  centre zero line: right helps this seat, left hurts it.
 *
 *  **Colour is not the only encoding.** Every row carries its signed number in
 *  a monospace column and sits on its own side of the zero line, so the chart
 *  reads correctly in greyscale, in forced-colors mode, and to a reader who
 *  cannot separate the two hues. The pair itself was validated rather than
 *  chosen by eye — blue/orange at ΔE 23.0 under protanopia against this
 *  surface, where the house green/red would have been the classic mistake. */
function Bars({
  terms,
  labels,
}: {
  terms: Explanation["terms"];
  labels: Labels | null;
}) {
  // The widest bar is the largest |product|, so the chart is scaled to what it
  // actually contains rather than to a constant nobody measured. A vector of
  // all zeros would divide by zero, hence the floor of 1.
  const peak = Math.max(1, ...terms.map((t) => Math.abs(t.product)));
  return (
    <div className="bars" role="list">
      {terms.map((t) => {
        const share = (Math.abs(t.product) / peak) * 100;
        const helps = t.product > 0;
        return (
          <div
            className="bar-row"
            role="listitem"
            key={t.feature}
            title={`${featureWords(t.feature, labels)} — you ${t.viewer_value}, best opponent ${t.opponent_value}, weight ${t.weight} → ${t.product}`}
          >
            <span className="bar-name">
              {featureWords(t.feature, labels)}
              <code>{t.feature}</code>
            </span>
            <span className="bar-track">
              <span className="bar-half left">
                {!helps && t.product !== 0 && (
                  <i className="bar hurts" style={{ width: `${share}%` }} />
                )}
              </span>
              <span className="bar-zero" aria-hidden="true" />
              <span className="bar-half right">
                {helps && <i className="bar helps" style={{ width: `${share}%` }} />}
              </span>
            </span>
            {/* PARENTHESISED, and that is a correctness fix rather than a
                flourish: `5 - 0 × 14` reads by precedence as `5 - (0 × 14)`,
                which is 5, not the 70 in the column beside it. The arithmetic
                is (yours − the best opponent's) × the weight, and the notation
                has to say so. */}
            <span className="bar-terms">
              ({t.viewer_value} − {t.opponent_value}) × {t.weight}
            </span>
            <span className={`bar-value ${helps ? "helps" : t.product < 0 ? "hurts" : "nil"}`}>
              {t.product > 0 ? `+${t.product}` : t.product}
            </span>
          </div>
        );
      })}
    </div>
  );
}

/** The whole explanation: the bars, the collapsed zeros, and the footed total.
 *
 *  **The total is not a caption, it is the proof.** It is printed under a rule
 *  as the sum of the column above it, so a reader can add the visible rows,
 *  add the hidden count's contribution of zero, and get the number the
 *  evaluator got. That is the difference between a chart and an explanation. */
export function ExplanationView({
  e,
  labels,
}: {
  e: Explanation;
  labels: Labels | null;
}) {
  const [showZeros, setShowZeros] = useState(false);

  // A FINISHED game is a different picture, not an empty one. Every product is
  // 0 there by design — an interpolated weight would be a lie about what the
  // evaluator used — and the whole score is the standing. Bars would render as
  // 34 empty rows under a nonzero total.
  if (e.terminal) {
    const t = e.terminal;
    return (
      <div className="explanation">
        <p className="muted">
          The game is over, so the positional features do not govern: this is a finished
          standing, and the evaluator scores it as one.
        </p>
        <dl className="standing">
          <div>
            <dt>Finishing place</dt>
            <dd className="num">
              {t.rank} of {t.seats}
            </dd>
          </div>
          <div>
            <dt>This seat's total</dt>
            <dd className="num">{t.viewer_total}</dd>
          </div>
          <div>
            <dt>Best other seat</dt>
            <dd className="num">{t.best_other_total}</dd>
          </div>
        </dl>
        <p className="total">
          <span>TOTAL</span>
          <span className="num">{e.total}</span>
        </p>
      </div>
    );
  }

  const sorted = [...e.terms].sort((a, b) => Math.abs(b.product) - Math.abs(a.product));
  const carrying = sorted.filter((t) => t.product !== 0);
  const silent = sorted.filter((t) => t.product === 0);
  const shown = showZeros ? sorted : carrying;

  return (
    <div className="explanation">
      <p className="axis-legend" aria-hidden="true">
        <span className="hurts">← hurts this seat</span>
        <span className="muted">0</span>
        <span className="helps">helps this seat →</span>
      </p>
      <Bars terms={shown} labels={labels} />
      <p className="total">
        <span>
          TOTAL
          <span className="muted">
            {" "}
            — {e.nonzero} of {e.terms.length} features carry a nonzero product
            {e.progress_ppm > 0 && `, weighted at ${(e.progress_ppm / 10000).toFixed(1)}% through the game`}
          </span>
        </span>
        <span className="num">{e.total > 0 ? `+${e.total}` : e.total}</span>
      </p>
      {silent.length > 0 && (
        <button type="button" className="ghost" onClick={() => setShowZeros((v) => !v)}>
          {showZeros
            ? `Hide the ${silent.length} features contributing nothing`
            : `Show the ${silent.length} features contributing nothing`}
        </button>
      )}
    </div>
  );
}

/** The line the search actually read the value at. Step 1 is the move itself. */
function Continuation({ line }: { line: string[] }) {
  if (line.length === 0) return null;
  return (
    <section>
      <h3>The line it read that value at</h3>
      <ol className="line">
        {line.map((step, i) => (
          <li key={`${i}-${step}`}>{step}</li>
        ))}
      </ol>
    </section>
  );
}

/** Runner-ups, by margin against the chosen move. A margin of 0 means TIED. */
function RunnerUps({ ranking }: { ranking: Ranked[] }) {
  const [all, setAll] = useState(false);
  if (ranking.length <= 1) return null;
  // **NOT `slice(1)`.** The ranking is sorted best-first, but the chosen move is
  // one of the MAXIMISERS and ties are broken by a keyed draw — so row 0 is not
  // reliably the move that was played. Measured on the committed fixture: two
  // candidates tie at 78 and the chosen one is row 1, so dropping row 0 listed
  // the played move as something it beat and hid a real runner-up. The server
  // marks the row, because inferring identity from a display string is a
  // display string doing a type's job.
  const rest = ranking.filter((r) => !r.chosen);
  const shown = all ? rest : rest.slice(0, RUNNER_UPS_SHOWN);
  const tied = ranking.filter((r) => r.margin === 0).length;
  return (
    <section>
      <h3>
        What it beat
        <span className="meta">
          margin against the suggestion, so 0 means tied
          {tied > 1 && ` — ${tied} candidates tie at the top`}
        </span>
      </h3>
      <ol className="runner-ups">
        {shown.map((r, i) => (
          <li key={`${i}-${r.text}`}>
            <span className={`bar-value ${r.margin === 0 ? "nil" : "hurts"}`}>{r.margin}</span>
            <span>{r.text}</span>
          </li>
        ))}
      </ol>
      {rest.length > shown.length && (
        <button type="button" className="ghost" onClick={() => setAll(true)}>
          Show all {rest.length} candidates
        </button>
      )}
    </section>
  );
}

/** The search's cost counters, grouped as the terminal groups them. */
function Counters({ counters }: { counters: Counter[] }) {
  const groups: [string, string][] = [
    ["work", "search cost"],
    ["limit", "guards"],
    ["ungrouped", "other"],
  ];
  return (
    <section>
      <h3>What the search spent</h3>
      {groups.map(([key, label]) => {
        const rows = counters.filter((c) => c.group === key);
        if (rows.length === 0) return null;
        return (
          <p className="counters" key={key}>
            <span className="meta">{label}</span>
            {rows.map((c) => (
              <span key={c.name}>
                <code>{c.name}</code>
                <b className="num">{c.value}</b>
              </span>
            ))}
          </p>
        );
      })}
    </section>
  );
}

/** A refusal, which is an ANSWER rather than an error.
 *
 *  The sentence is the CLI's own — the same words `guide_sync` pins to the
 *  tester guide — so the browser and the terminal cannot refuse the same thing
 *  differently. The tag is shown too: it is what a bug report should quote. */
function Refused({ refusal }: { refusal: RefusalRow }) {
  return (
    <div className="refusal">
      <code>{refusal.kind}</code>
      <p>{refusal.message}</p>
    </div>
  );
}

export function SuggestView({
  data,
  labels,
}: {
  data: SuggestResponse;
  labels: Labels | null;
}) {
  if (!data.ok) {
    return data.refusal ? <Refused refusal={data.refusal} /> : null;
  }
  const s = data.ok;
  return (
    <div className="explain">
      <section>
        <h3>It would play</h3>
        <p className="headline">{s.text}</p>
        {s.tied_at_top > 1 && (
          <p className="meta">
            {s.tied_at_top} candidates score exactly this, and the tie was drawn from the
            decision's own key — not from whichever the generator listed first.
          </p>
        )}
      </section>
      <section>
        <h3>Why</h3>
        <p className="meta">
          Each row is the bot's own arithmetic: (your value − the best opponent's) × the
          weight for this stage of the game. It read ONE guess at the hidden cards, and its
          weights were never tuned on human play.
        </p>
        <ExplanationView e={s.explanation} labels={labels} />
      </section>
      <Continuation line={s.continuation} />
      <RunnerUps ranking={s.ranking} />
      {s.search_hit_ply_cap && (
        <p className="warn">
          The ply guard stopped some continuation inside this search — not necessarily the
          one above. The counter carries no identity, so which line it stopped is not
          recorded.
        </p>
      )}
      <p className="meta provenance">
        Stream <code>{s.stream_seed}</code> — the key this press was made under. Nothing was
        applied, recorded or saved.
      </p>
    </div>
  );
}

export function WhyView({ data, labels }: { data: WhyResponse; labels: Labels | null }) {
  if (!data.ok) {
    return data.refusal ? <Refused refusal={data.refusal} /> : null;
  }
  const w = data.ok;
  const p = w.provenance;
  const drifted = p.recorded_version !== null && p.recorded_version !== p.current_version;
  return (
    <div className="explain">
      <section>
        <h3>Seat {p.seat} played</h3>
        <p className="headline">{w.text}</p>
        {p.forced && (
          <p className="meta">
            The engine offered exactly one move here, so this is an account of a move nothing
            else was competing with.
          </p>
        )}
      </section>
      {drifted && (
        <p className="warn">
          The record stores this controller at version {p.recorded_version} and this build runs
          version {p.current_version}. The move still reproduces, so what follows is THIS
          build's reading of a move an older one played.
        </p>
      )}
      <section>
        <h3>Why</h3>
        <ExplanationView e={w.explanation} labels={labels} />
      </section>
      <Continuation line={w.continuation} />
      <RunnerUps ranking={w.ranking} />
      {w.search_hit_ply_cap && (
        <p className="warn">
          The ply guard stopped some continuation inside this search — not necessarily the
          one above. The counter carries no identity, so which line it stopped is not
          recorded.
        </p>
      )}
      {w.counters && <Counters counters={w.counters} />}
      <p className="meta provenance">
        Reconstructed by replay under this build: <code>{p.contestant}</code>, evaluator impl{" "}
        {p.evaluator_version}, from decision seed{" "}
        <code>{p.decision_seed}</code> — the key the bot's own pump derived. Event {p.index}.
        A reconstruction that did not equal the recorded move would have refused instead of
        showing you this.
      </p>
    </div>
  );
}
