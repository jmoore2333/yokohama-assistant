# Could the browser client live on perpetually-free cloud compute?

**Question asked (owner, 2026-09-01):** if the app ran on free cloud compute —
Google Cloud's perpetual free tier, AWS, or Alibaba — with no Cloudflare Access,
a lobby/multi-user model, and a periodic session drop accepted, could a fully
free service handle it?

**Answer: yes, comfortably, and the machine was never the question.** Every
resource axis is non-binding by one to four orders of magnitude. The decision is
entirely about *terms* — perpetuity, what happens at the limit, and who holds a
credit card — plus one blocker that has nothing to do with hosting (§6).

**Status: research, not a decision.** Nothing here is deployed. The NAS is still
serving the product and `deploy/RUNBOOK.md` §3.3 says it should go on doing so.

Sources: five parallel research agents against first-party provider
documentation, 2026-09-01. Where a provider's own pages contradict each other or
a widely-repeated figure is blog-sourced, that is called out — several do.

---

## 1. The workload, measured (not estimated)

Every figure below was measured on this branch, not inferred from the framework.

| | Reading | Where |
|---|---|---|
| Resident memory, idle → after two bot searches | **1.0 MB → 1.2 MB** | RUNBOOK §3.3 |
| `suggest`, full 186-candidate search, 2014 Atom | **0.45 s / 0.57 s** | RUNBOOK §3.3 |
| Container image | **19 MB** | RUNBOOK §3.3 |
| Binary, x86_64 musl / aarch64 musl | **5.8 MB / 5.8 MB**, both static | RUNBOOK §1, §1.1 |
| First paint (html + js + css) | **277 KiB** | RUNBOOK §3.4 |
| Session ceiling (all art + help opened) | **~5 MB** | RUNBOOK §3.4 |

**Consequences, stated so they are not re-litigated:**

- **RAM is irrelevant.** The smallest free instance anywhere is 512 MB. We use 1.2 MB.
- **CPU is nearly irrelevant.** Half a second of a 2014 Atom core per search.
- **Architecture is irrelevant.** `aarch64-unknown-linux-musl` cross-compiles in
  16.3 s with no C dependencies in the tree, so ARM-only free tiers — Oracle
  Ampere, Azure's B2pts v2, Graviton — are *fully eligible*. Do not eliminate a
  provider for this.
- **Egress is provider-dependent and is the one resource axis that can bind.**
  See §3. This corrects an earlier over-broad claim of ours.

---

## 2. The comparison

Perpetuity is the first column because it is what the question asked about.
**"Perpetual"** = renews monthly for as long as the account exists.
**"12-month"** / **"trial"** = expires.

| Option | Perpetual? | Disk | Sleep / cold start | Card? | At the limit |
|---|---|---|---|---|---|
| **Oracle Ampere A1 VM** | **Perpetual** | **200 GB block, persistent** | None — always-on VM | Yes (not charged unless upgraded) | **Hard stop.** Instance disabled, deleted after 30 days |
| **Oracle AMD E2.1.Micro ×2** | **Perpetual** | Same 200 GB pool | None | Yes | Hard stop |
| **Azure Container Apps** | **Perpetual** (product grant, per subscription) | Ephemeral | Scale-to-zero default | Yes | **Silent bill** — see §4 |
| **GCP Cloud Run** | **Perpetual** | Ephemeral | Scale-to-zero, $0 idle | Yes | Silent bill, **but a real spend cap exists** (Preview) |
| **GCP e2-micro VM** | **Perpetual** | 30 GB pd-standard | None | Yes | Silent bill, no spend cap for Compute Engine |
| **Azure B2pts v2 (ARM, 2 vCPU)** | 12-month | — | None | Yes | — |
| **AWS EC2 / Lambda** | Legacy accounts only | `/tmp`, per-environment | Lambda cold starts | Yes | **Bill, or account deletion** — see §5 |
| **Alibaba Cloud** | **None** | — | — | — | Trials only, 3 months |
| **Render** | Perpetual, 750 h/mo | Ephemeral | ~60 s wake after 15 min idle | **No** | **Hard stop** |
| **Railway** | Perpetual (trial credit) | **0.5 GB persistent** | 5–10 min → sleep | **No** | **Hard stop** |
| **Koyeb** | Perpetual | Ephemeral | 1–5 s wake | Yes | Silent bill |
| **Fly.io** | Not since Oct 2024 | — | ~2 s (official) | Yes | Silent bill |
| **Vercel / Deno Deploy** | Hobby / free | — | — | No | Hard stop / **can't run a Rust binary** |

---

## 3. Egress — the one resource that can actually bind, and it is Google's

This corrects RUNBOOK §3.4, which originally generalized "egress is not a
constraint" from a 100–200 GB assumption. Per provider:

| Provider | Free egress/month | Sessions at our ~5 MB ceiling |
|---|---|---|
| **Oracle** | **10 TB** | Effectively unlimited |
| **Azure** | 100 GB (perpetual, all customers, all regions) | ~20,000 |
| **AWS** | 100 GB | ~20,000 |
| **GCP e2-micro / Cloud Run** | **1 GB (North America)** | **~200** — or ~3,700 first paints |

Google is the outlier by four orders of magnitude against Oracle. For "a handful
of users" 1 GB still works, but it is a real ceiling, and it is the one number
that would bite without warning. Putting Cloudflare in front to cache the static
assets removes it — the art and the JS bundle are the entire payload, and they
are immutable per build.

---

## 4. What actually happens when you exceed a limit

This is the column that should drive the decision, because it is where an
unattended hobby deployment turns into a bill.

**Oracle is the only provider where being billed is structurally impossible.**
On a never-upgraded Always Free tenancy: *"Your credit card will not be charged
unless you upgrade your account."* Over-limit instances are disabled, then
deleted after 30 days. On PAYG the failure mode inverts and it bills the
overage — so if you take Oracle, **stay on pure Always Free**, and know that the
same protection is what makes it possible to lose the instance.

**Google, Azure and AWS all silently bill on their perpetual paths.** Google's
own FAQ: *"If you have a Paid billing account and go beyond those limits, then
you will be billed for the overage."* Budgets are alerts, not caps —
*"Setting an alerts-only budget doesn't automatically cap … usage or spending."*

**One genuine exception, and it is Google's:** *spend cap budgets* (Preview,
Cloud Run support since 2026-07-27) actually block new usage when triggered.
**Compute Engine is not eligible** — so this favours Cloud Run over the
e2-micro, and it is the single strongest argument for Cloud Run in this survey.

**Azure has a trap worth stating plainly.** The free account's spending limit
hard-stops — but it *"isn't available for subscriptions with … pay-as-you-go
pricing"*, and you must move to pay-as-you-go within 30 days to keep receiving
free services at all. **The perpetual-free Azure path necessarily means an
auto-billing subscription with no spending limit.**

**Render, Railway, Vercel and Deno hard-stop and take no card** — the only
options where the worst case is downtime rather than money.

---

## 5. AWS is the worst fit of the majors

Worth stating because the question named it. Three separate problems:

1. **The new Free plan hard-stops by deleting your account.** *"Your account
   closes automatically, and you lose access to your resources and data"* after
   six months or when credits are exhausted (90-day recovery window). That is
   not a safety mechanism to rely on.
2. **The Paid plan silently bills** with alerts at 85% and nothing that stops.
3. **Lambda breaks our state model, and this is the real blocker.** Game records
   are JSON files on local disk; on Lambda that is `/tmp`, which is
   per-execution-environment and *"terminated every few hours … even for
   functions that are invoked continuously."* Two concurrent players can hit two
   environments holding **different** state. That is not the acceptable "drops
   sessions" behaviour — it is silent divergence, which is worse than loss.

EC2's 750 h/month is 12-month-only for accounts created after 2025-07-15.

**Alibaba has no always-free compute at all** — ECS is $90 of credit over three
months, Function Compute is a first-time-user trial for three months. The
"one million calls per month, no expiration" figure still in Alibaba's own older
community blogs describes a tier that no longer appears in current billing docs.

---

## 6. Oracle's June 2026 halving — verified, because it decides the front-runner

Oracle is the strongest option on paper, so its terms were checked directly
rather than trusted to secondary sources. **The halving is confirmed.**

Oracle's own Always Free page, *"Updated 2026-06-12"*: the allowance is now
*"the first 1,500 OCPU hours and 9,000 GB hours per month … For Always Free
tenancies, this is equivalent to 2 OCPUs and 12 GB of memory."* The former
"up to 4 instances" language is gone. There was no announcement, no email, no
blog post — the change was noticed as a documentation diff, and enforcement
began 2026-08-18.

**Two of Oracle's own live pages still say 3,000/18,000** (the Compute ARM
reference and the OCI Price List), as does essentially every third-party guide
written before mid-2026. **Plan against 2 OCPU / 12 GB** — which is still four
times more than this workload needs.

**Two claims that did *not* survive checking, recorded so they are not repeated:**

- The widely-quoted *"if an existing resource is terminated, it may not be
  possible to recreate resources above the updated limit"* traces only to an
  InfoQ community-reaction passage and its syndication. It is **not** in
  Oracle's docs or FAQ. Unverified community inference, not policy.
- Claims that Oracle eliminated all egress charges in Feb 2026 do not hold up;
  three official pages still state the 10 TB limit.

**The real Oracle risk is not the quota, it is availability.** Forum reports
include a tenancy at exactly 2 OCPU / 12 GB disabled anyway in August, and idle
reclamation stops instances with a *manual* restart that is conditioned on
*"the associated compute shape [being] available in your region."* So the
failure mode is an outage lasting until you notice **and** until A1 capacity
frees up. Whether PAYG retains 4/24 is unresolved; Oracle has published nothing
either way.

---

## 7. Recommendation

**If the goal is a genuinely free always-on host: Oracle Ampere A1.** It is the
only option that is perpetual, always-on (no cold start), has real persistent
disk (200 GB), has egress that will never bind (10 TB), and *cannot bill you*.
Our aarch64 build lands on it directly. Accept that instances are occasionally
disabled without warning and that recovery waits on regional capacity — this is
a hobby deployment where that is survivable.

**If the goal is zero billing risk above all: Render.** No card, hard stop,
perpetual. Pay for it with a ~60 s wake after 15 minutes idle, which our small
image will *not* meaningfully improve — that is a full instance re-provision.

**If you want Google specifically: Cloud Run, not the e2-micro.** Scale-to-zero
is genuinely $0 at this traffic (two searches a day is 0.03% of the grant), and
it is the only place with a spend cap that actually blocks. Watch the 1 GB
egress and know the disk is ephemeral. The e2-micro additionally carries an
external IPv4 charge of roughly **$3.65/month** that the free-tier page never
mentions — so the "$0 VM" is not $0 unless it is IPv6-only.

**Not AWS, and not Alibaba,** for the reasons in §5.

---

## 8. The blocker that is not about hosting

**The server has no authentication of any kind.** Verified at HEAD, not from
memory: 16 routes, zero `.layer()`, no session, no header check. `GET /api/games`
enumerates every game id, and `DELETE /api/games/{id}` is unauthenticated.

Today that is safe only because Cloudflare Access is in front of it. In the
scenario this document is about — a free host, no Access — the game store is not
merely publicly readable, it is **publicly writable and publicly deletable by
anyone who finds the URL.**

So the lobby/multi-user work is not a parallel nicety to the hosting move. It is
a **precondition** for it, and it has to land *before* Access comes off, not
alongside. Any of the options above is fine to *experiment* on behind Access
first; none of them is fine to expose without that work.
