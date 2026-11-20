// **The README sidecar's redaction** (Task 8, fix round 1, review I-1).
//
// `readme_shots.spec.ts` photographs each page with every lobby row that is not
// one of the run's own tables MASKED: live, the lobby is shared, and a
// stranger's table with its seat addresses must never reach a picture. The page's
// accessibility snapshot, written beside the picture as text, is read by an
// agent in the picture's place, so it must leave out exactly what the picture
// masks. This is that step, as a pure function a node cell holds
// (`aria_redact.node.test.ts`); the spec collects the masked rows' links from
// the page at the moment it shoots, and hands them here.
//
// Two halves, neither the other's fallback:
//
//   1. **Redaction, keyed the way the mask is**: by the row's `#/t/<id>` link.
//      Every `listitem` block of the snapshot whose `/url` is a masked row's
//      href is replaced by one line, `- listitem: [masked: not this run's
//      table]` at the row's indent, so the row's name, link, seat labels and
//      button names all go and the list still reads as having had a row there.
//   2. **A guard over what is left**: any `#/t/<id>` that is not one of the
//      run's own tables, or any e-mail address that is not one of the run's
//      people, throws before the file is written. That is what stops a new
//      kind of row the mask does not know about from slipping through silently.
//      Its message counts what it found and NEVER names it, since a test log is
//      read too.

export const MASKED_ROW = "[masked: not this run's table]";

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

/** The text to write beside a README picture: `snapshot` with every row whose
 *  link is in `maskedHrefs` left out, refused if anything foreign survives. */
export function redactSidecar(snapshot: string, maskedHrefs: readonly string[], own: Own): string {
  const text = redactRows(snapshot, maskedHrefs);
  guard(text, own);
  return text;
}
