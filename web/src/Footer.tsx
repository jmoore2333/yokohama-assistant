// The one-line notice at the foot of every screen (M6.5c2, R-6.5c2.40).
//
// **Mounted in `main.tsx` after `<App/>`, not inside it**: `App` has about
// fifteen `return` sites and no wrapper. At 1201px and wider the game screen's
// `.app` is a grid of the viewport's height, so a footer drawn below it would
// sit below the fold; `has-notice` on the root element lets the stylesheet take
// the footer's height off `.app` (`--notice-h`), and the table's fit rule
// follows. Where the build set no notice nothing is drawn and nothing is marked.

import { useEffect } from "react";
import { siteNotice } from "./buildValues";

export function Footer() {
  const notice = siteNotice();
  useEffect(() => {
    if (notice === null) return;
    const root = document.documentElement;
    root.classList.add("has-notice");
    return () => root.classList.remove("has-notice");
  }, [notice]);
  if (notice === null) return null;
  return <footer className="site-notice">{notice}</footer>;
}
