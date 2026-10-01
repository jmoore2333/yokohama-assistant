import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { turnstileSiteKey } from "./buildValues";
import { Challenge } from "./Challenge";
import { Footer } from "./Footer";
import { captureInstallPrompt } from "./install";
import "./styles.css";

// **Listening for `beforeinstallprompt` BEFORE the first render** (M6.5b2,
// R-6.5b2.9). Chromium fires it once, early, and a listener attached when the
// door mounts can miss it — so the entry point is where it is armed, and the
// door's own hook asks again idempotently for the pages that never come
// through here (every component cell).
captureInstallPrompt();

// **The edge's check, mounted once beside the app and only under a site key**
// (cycle F, R-F6; spec §2.6): the overlay is the seam's one handler, so a build
// with no key has none, and a request the edge challenges is the ordinary
// refusal at once, naming the edge. Mounting it loads nothing: the widget's
// script is asked for when a challenge opens it.
const challenge = Boolean(turnstileSiteKey());

const root = document.getElementById("root");
if (!root) throw new Error("index.html must carry a #root element");
createRoot(root).render(
  <StrictMode>
    <App />
    <Footer />
    {challenge && <Challenge />}
  </StrictMode>,
);
