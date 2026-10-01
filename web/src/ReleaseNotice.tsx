// The one line that says a newer release exists (M6.5c2 cycle F, R-F2, R-FP9;
// spec §2.2): "A newer release is available: v0.7.0. This site runs 0.6.0".
//
// **Two places, one component**: on the door under the blurb (above the project
// link), and in the signed-in lobby's actions beside the project link. It draws
// nothing until the feed has answered, and nothing for an answer that is not
// newer — the reading is `useReleaseNotice`'s (`releaseFeed.ts`), which asks the
// feed at most once per page life. The tag is the link, to the release's own page
// (the feed's `html_url`, else the project link's releases page) in a new tab;
// with nowhere to link it is plain text. There is no trailing period — the update
// line has none either — and the line is information: it offers nothing to click
// but the release's own page, and nothing is taken or reloaded by it.

import { buildVersion } from "./buildValues";
import { useReleaseNotice } from "./releaseFeed";

export function ReleaseNotice({ fetchImpl }: { fetchImpl?: typeof fetch }): React.ReactNode {
  const notice = useReleaseNotice(fetchImpl);
  if (notice === null) return null;
  return (
    <p className="muted release-notice">
      A newer release is available:{" "}
      {notice.url === "" ? (
        notice.tag
      ) : (
        <a href={notice.url} target="_blank" rel="noopener">
          {notice.tag}
        </a>
      )}
      . This site runs {buildVersion()}
    </p>
  );
}
