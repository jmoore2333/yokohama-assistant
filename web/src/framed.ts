// **Not inside another page** (security S2a, finding B4's third gap; ruling A5). The host's
// `X-Frame-Options: DENY` is what keeps a signed-in page out of a hostile frame; this is the client's own
// refusal for a frame that was rendered anyway, and it costs one comparison at boot.

/** Whether this document is inside another page's frame: its window is not the top one. A `top` of `null`
 *  (a detached frame) reads as framed too — the safe side. */
export function framed(win: { top: unknown; self: unknown }): boolean {
  return win.top !== win.self;
}

/** What the page says instead of rendering, when it is framed. */
export const FRAMED_SENTENCE = "Yokohama does not run inside another page. Open it in a tab of its own.";
