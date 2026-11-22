// **The README sidecar's redaction** (Task 8, fix round 1, review I-1).
//
// `readme_shots.spec.ts` photographs each page with every lobby row that is not
// one of the run's own tables MASKED: live, the lobby is shared, and a
// stranger's table with its seat addresses must never reach a picture. The page's
// accessibility snapshot, written beside the picture as text, is read by an
// agent in the picture's place, so it must leave out what the picture masks —
// and every chat log besides, which the picture does not show and the text
// would (the whole-branch review, W-1): a folded chat panel is clipped to
// nothing (`max-height: 0; overflow: hidden`), not hidden from the
// accessibility tree, so every message it holds is in the snapshot, and live a
// house chat's messages are strangers' names and free text. This is that step,
// as a pure function a node cell holds
// (`aria_redact.node.test.ts`). The spec takes the screenshot, then the
// snapshot, then reads the masked rows' links off the page, and hands the last
// two here. That order is the safe one: a row that arrives between the snapshot
// and the link read is in the list and not in the text, which costs nothing; a
// row that leaves in between keeps its link in the text with nothing to redact
// it, and the guard below refuses the file.
//
// Three steps, in this order, the last no other's fallback:
//
//   1. **Every chat log goes whole**: each `log` block of the snapshot (the log
//      line and every line indented under it) becomes one line, `- log:
//      [masked: chat]`, at its indent. The run posts no chat, so nothing an alt
//      needs is lost; and it goes BEFORE the guard, so a stranger who quotes a
//      table link in the chat cannot make the guard refuse the run.
//   2. **Redaction, keyed the way the mask is**: by the row's `#/t/<id>` link.
//      Every `listitem` block of the snapshot whose `/url` is a masked row's
//      href is replaced by one line, `- listitem: [masked: not this run's
//      table]` at the row's indent, so the row's name, link, seat labels and
//      button names all go and the list still reads as having had a row there.
//   3. **A guard over what is left**: any `#/t/<id>` that is not one of the
//      run's own tables, or any e-mail address that is not one of the run's
//      people, throws before the file is written — the id matched exactly and
//      the address exactly, ignoring case. That is what stops a new kind of row
//      the mask does not know about from slipping through silently, as long as
//      it carries a table link or an address. **It sees nothing else**: a
//      person's name or a line of free text is invisible to it, which is why
//      the chat goes whole in step 1 rather than being left to the guard. Its
//      message counts what it found and NEVER names it, since a test log is
//      read too.

export const MASKED_ROW = "[masked: not this run's table]";
export const MASKED_CHAT = "[masked: chat]";

/** What the run made: its tables' ids and its people's addresses. */
export interface Own {
  ids: Iterable<string>;
  addresses: Iterable<string>;
}

const indentOf = (line: string): number => line.length - line.trimStart().length;

/** The `/url` value of an aria-snapshot line, quoted or not, else null. */
function urlOf(line: string): string | null {
  const m = /^\s*- \/url: (?:"([^"]*)"|(\S+))\s*$/.exec(line);
  return m ? (m[1] ?? m[2] ?? null) : null;
}

/** The end (exclusive) of the block whose first line is `lines[i]`: every
 *  following line indented deeper than it, up to a blank line. */
function blockEnd(lines: readonly string[], i: number): number {
  const indent = indentOf(lines[i] ?? "");
  let end = i + 1;
  while (end < lines.length && (lines[end] ?? "").trim() !== "" && indentOf(lines[end] ?? "") > indent) end++;
  return end;
}

/** Replace every `log` block (a chat panel's messages, its box and its send
 *  button) with one `- log: [masked: chat]` line at its indent. */
function redactLogs(snapshot: string): string {
  const lines = snapshot.split("\n");
  const out: string[] = [];
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i] ?? "";
    if (/^\s*- '?log\b/.test(line)) {
      out.push(`${" ".repeat(indentOf(line))}- log: ${MASKED_CHAT}`);
      i = blockEnd(lines, i) - 1;
    } else {
      out.push(line);
    }
  }
  return out.join("\n");
}

/** Replace every `listitem` block holding a link to one of `maskedHrefs`. */
function redactRows(snapshot: string, maskedHrefs: readonly string[]): string {
  if (maskedHrefs.length === 0) return snapshot;
  const masked = (url: string) => maskedHrefs.some((href) => url === href || url.endsWith(href));
  const lines = snapshot.split("\n");
  const at = (k: number): string => lines[k] ?? "";
  const out: string[] = [];
  for (let i = 0; i < lines.length; i++) {
    const line = at(i);
    if (!/^\s*- '?listitem\b/.test(line)) {
      out.push(line);
      continue;
    }
    const indent = indentOf(line);
    let end = i + 1;
    while (end < lines.length && at(end).trim() !== "" && indentOf(at(end)) > indent) end++;
    const block = lines.slice(i, end);
    if (block.some((l) => {
      const url = urlOf(l);
      return url !== null && masked(url);
    })) {
      out.push(`${" ".repeat(indent)}- listitem: ${MASKED_ROW}`);
      i = end - 1;
    } else {
      out.push(line);
    }
  }
  return out.join("\n");
}

const TABLE_ID = /#\/t\/([^"'\s/?#]+)/g;
const ADDRESS = /[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+/g;

/** Throws if `text` names a table or an address the run did not make. */
function guard(text: string, own: Own): void {
  const ids = new Set(own.ids);
  const addresses = new Set([...own.addresses].map((a) => a.toLowerCase()));
  const foreignIds = [...text.matchAll(TABLE_ID)].filter((m) => !ids.has(m[1] ?? "")).length;
  if (foreignIds > 0) {
    throw new Error(`readme_shots: the sidecar still holds a table id that is not this run's (${foreignIds} in all) — the mask and the redaction disagree; nothing was written`);
  }
  const foreignAddresses = [...text.matchAll(ADDRESS)].filter((m) => !addresses.has(m[0].toLowerCase())).length;
  if (foreignAddresses > 0) {
    throw new Error(`readme_shots: the sidecar still holds an address that is not one of this run's people (${foreignAddresses} in all) — nothing was written`);
  }
}

/** The text to write beside a README picture: `snapshot` with every chat log
 *  and every row whose link is in `maskedHrefs` left out, refused if a table id
 *  or an address the run did not make survives. */
export function redactSidecar(snapshot: string, maskedHrefs: readonly string[], own: Own): string {
  const text = redactRows(redactLogs(snapshot), maskedHrefs);
  guard(text, own);
  return text;
}
