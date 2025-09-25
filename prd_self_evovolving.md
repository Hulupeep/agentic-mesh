# AMP Self‑Evolving Platforms — Product Requirements Document (PRD) v3

## 0) TL;DR
**Software ossifies at requirements time.** The repo reflects **historical needs**, not today’s signals. **AMP** makes products **self‑evolve** (and self‑heal) using **outer awareness** (data, users, environments) and **inner awareness** (their own traces, SLOs, budgets). Changes are proposed, validated (Shadow/A‑B), safely deployed, and fully audited — automatically.

---

## 1) Problem
- **Fixed‑in‑time software:** Most features are specified 3–12 months prior; code embodies stale assumptions. Real usage shifts faster than backlogs.
- **Blind to signals:** Apps rarely close the loop between **outer signals** (source content, traffic mix, device, geography, seasonality) and **inner signals** (latency, error, cost, satisfaction, conversion).
- **Manual evolution is slow:** PMs “discover” drift weeks later, file tickets, wait for cycles. Meanwhile **cost/latency creep** and **quality regressions** compound.
- **Glue fatigue:** Teams rebuild the same bespoke pipelines (retrieval, retries, caching, metrics) per use case; none are portable or audited.

**Core insight:** The product should evolve **in flight** — safely and audibly — instead of waiting for the next sprint.

---

## 2) Mission
Enable **any SaaS platform** to **self‑evolve and self‑heal**: listen to signals, propose the smallest safe change, **validate**, **deploy**, and **rollback** if needed — all within **evidence**, **policy**, and **budget** guardrails.

---

## 3) Product Definition
**AMP** is a thin orchestration layer where:
- **Plans are code:** small, inspectable flows that compose **search → grounding (evidence/citations) → memory** (and other app actions) per request/journey.
- **Tools are the ABI:** each tool exposes clear inputs/outputs, constraints, and cost/latency expectations.
- **Evidence is the gate:** answers and state changes require supporting evidence/confidence; contradictions are detected.
- **Memory is gated state:** only proven facts persist, with TTL and provenance.
- **Change‑Proposals**: typed, auditable change requests generated from signals (knobs, rationale, expected deltas, risk class, rollback rules).
- **Self‑Evolution loop (ODPVD‑R):** Observe → Diagnose → Propose → Validate (Shadow/A‑B) → Deploy → Rollback.

**Scope:** Works for **grounded answers** and **entire platforms** (onboarding, activation, pricing/packaging, search/recs, abuse/fraud, reliability/cost ops, and classic CRUD apps like to‑do).

---

## 4) Principles
- **Evidence‑first:** Prove it or say you can’t. Don’t persist unproven memory.
- **Budget‑bounded:** Every change respects p95 latency and cost targets.
- **Risk‑tiered autonomy:** Low‑risk changes self‑apply; Medium/High require approval windows.
- **Auditability by default:** Signed traces, replay bundles, change logs.
- **Portability, not lock‑in:** Thin contracts layer atop existing stacks.

---

## 5) Outcomes & Business KPIs
- **Trust:** Citation rate ≥ **90%**; contradictions ≤ **3%** (for answer flows).
- **Speed:** p95 ≤ **2.0s** for key journeys; at least one validated improvement of **p95 −35%** in quarter.
- **Cost:** Cost/journey within **±10%** of baseline after warm‑up; highlight fast‑path savings.
- **Growth:** Activation/conversion **+8–15%** where AMP is applied to journeys.

---

## 6) Personas & JTBD
- **Founder/PM:** Keep product aligned with live usage, not 6‑month‑old assumptions.
- **Support Lead:** Ship *cited*, policy‑safe answers; deflect tickets.
- **CTO/CISO:** Enforce budgets and evidence; get audit trails.
- **SRE/Ops:** Reduce MTTR; keep reliability within SLOs.
- **End‑User:** Faster, more accurate outcomes that adapt to context.

---

## 7) Use Cases (Representative)
### 7.1 Grounded Answers (wedge)
Return cited, policy‑safe answers from customer documents/systems with budget control and self‑evolution (tuning retrieval, thresholds, caching) to maintain SLOs.

### 7.2 Platform Journeys
- **Onboarding/Activation:** Reorder steps, copy aids, defer paywalls when signals show friction.
- **Pricing/Packaging:** Adjust trial length, feature gating, usage caps by segment under SLO/ROI bounds.
- **Search/Recommendations:** Tune retrieval breadth (`k`), rerank strategy, freshness bias; cache fast paths.
- **Reliability/Cost Ops:** Switch backends, tune cache TTL, enforce per‑journey budgets.

### 7.3 End‑to‑End Example — **To‑Do App that Self‑Evolves**
**Scenario:** A simple to‑do SaaS shows a **“Add Task”** button in the footer.
- **Synthetic signal for demo:** For this example, each local button press is treated as **5,000 production‑equivalent presses** (press weight = 5,000). The user clicks the button **5 times** → system treats it as **25,000 presses**.

**Observation (inner & outer awareness):**
- Spike in `Add Task` presses at mobile breakpoints; elevated p95 on task creation; user path shows multiple taps to reach the button.

**Diagnose:**
- High friction to add tasks in common contexts (keyboard focus lost; button placement suboptimal). Cost impact minimal; latency slightly elevated on mobile due to re‑render.

**Propose (Change‑Proposal auto‑generated):**
- **Scope:** To‑Do UI plan macro + input handling.
- **Diff:**
  1) Add **“Enter” to add** when cursor in title field.
  2) Promote **Add Task** to sticky top‑right on mobile.
  3) Batch‑insert if user adds >3 tasks within 10s (merge into one network call).
- **Expected deltas:** p95 **−35%** for add flow on mobile; task creation rate **+12%**; API calls **−30%**.
- **Risk class:** **Low** (no side effects beyond UI/routing); **Rollback:** restore prior layout + disable batch.

**Validate:**
- **Shadow test:** replicate last week’s traffic with proposed plan; require p95 **≤ target** and no regression in error rate.
- **A‑B test:** 10% traffic slice; **pass if** task creation **+≥8%** and p95 within budget; cost **≤ +5%**.

**Deploy:**
- Auto‑apply (Low‑risk). Change log entry created; signed traces link to proposal and metrics.

**Rollback rules:**
- If creation rate < control or user errors rise by >2%, revert and flag Medium‑risk review.

**Audit trail (auto‑generated):**
- Proposal ID, diff, rationale, validation metrics, decision, rollback window, and replay bundle.

**Success snapshot:**
- After 24h, p95 on add flow −32% (close to target), creation +9%, API calls −27%. System proposes retaining shortcut & sticky placement; keep monitoring batch threshold.

---

## 8) Functional Requirements (What must exist)
- **FR‑1 Plans as Code:** Represent journeys as small plans (JSON‑style graphs) that call tools, branch, assert, write memory.
- **FR‑2 Tool Contracts:** Each tool declares I/O, constraints, and cost/latency expectations.
- **FR‑3 Evidence & Grounding:** For claim‑like outputs, compute confidence, citations, and contradictions; block or degrade if under policy.
- **FR‑4 Memory Hygiene:** Persist only when evidence ≥ threshold; attach provenance and TTL; support forget/quarantine.
- **FR‑5 Budgets:** Accept per‑request latency and cost caps; predict/track adherence.
- **FR‑6 Traces & Replay:** Emit signed step traces; package replay bundles (plan + versions + traces) for audits.
- **FR‑7 Change‑Proposals:** Auto‑generate typed diffs with rationale, expected deltas, risk class, and rollback plan.
- **FR‑8 Validation Harness:** Run Shadow and/or A‑B automatically; gate deployments by pass criteria.
- **FR‑9 Risk‑Tiered Deployment:** Auto‑apply Low‑risk; require approvals for Medium/High; support emergency stop.
- **FR‑10 Dashboards:** SLO view for quality, latency, cost, and evolution history; per‑tenant/per‑journey filters.

---

## 9) Non‑Functional Requirements (SLOs)
- **Quality:** GQS ≥ **0.80** weekly; citation rate ≥ **90%**; contradictions ≤ **3%** (where applicable).
- **Latency:** p50 ≤ **800ms**, p95 ≤ **2.0s** for key journeys (90th percentile tenant load).
- **Cost:** Cost/journey within **±10%** of baseline after 7‑day warm‑up.
- **Availability:** **99.5%** API; degraded mode fails closed (no uncited claims, no risky changes).
- **Evolution Cadence:** ≥ **1** validated Low‑risk auto‑change per active tenant per week.

---

## 10) Operating Model — ODPVD‑R
- **Observe:** Collect traces; compute SLOs (quality, latency, cost, growth). Detect anomalies/drift.
- **Diagnose:** Attribute to causes (source freshness, device mix, layout friction, backend degradation).
- **Propose:** Emit structured **Change‑Proposals** (knobs, rationale, expected deltas, risk, rollback).
- **Validate:** Shadow and/or A‑B with **pass gates** (e.g., **p95 −35%**, **+≥8%** conversion, cost **≤ +10%**).
- **Deploy:** Auto for Low‑risk; approval for Medium/High.
- **Rollback:** Automatic if post‑deploy SLOs regress.

---

## 11) Governance & Safety
- **Risk Classes:** Low (UI/threshold/caching), Medium (backend swaps among approved), High (side‑effects, legal/medical domains, policy thresholds).
- **Human Override:** Always available; emergency stop restores last known‑good plan.
- **Compliance:** Every change linked to proposal, validations, and traces. Exportable audits.

---

## 12) Data, Privacy, and Compliance
- **Provenance** stored with facts; **TTL** and right‑to‑forget supported.
- **Tenant isolation** for indexes and memories.
- **Fail‑closed** policies for unproven outputs.

---

## 13) Observability
- **SLO Dashboard:** GQS, citations, contradictions, p50/p95, cost/journey, evolution history.
- **Traces:** Signed step events; downloadable NDJSON.
- **Replay:** Bundle of plan + tool versions + traces; reproduces decisions and budgets.

---

## 14) Rollout & Milestones
- **M0 (Week 1):** Grounded Answers + budgets + traces; manual dashboard.
- **M1 (Week 2):** Self‑Evolution Beta (critic + Change‑Proposals + Shadow/A‑B + Low‑risk auto‑apply). 
- **M2 (Weeks 4–6):** Platform journeys (Onboarding, To‑Do demo), multi‑tenant billing, SLA reports, approval flows → GA.

---

## 15) Pricing & Packaging (Wedge → Platform)
- **Starter** €49/mo — 1 source, 1k verified answers, basic dashboard.
- **Pro** €199/mo — 5 sources, 10k answers, Self‑Evolution Beta, audits.
- **Platform add‑on** — enable journey evolution (onboarding/pricing/to‑do) on Pro/Business tiers.
- **Guarantee:** *Citations or free* for answer flows; *SLO‑guarded experiments* for journeys.

---

## 16) Risks & Blind Spots (and Mitigations)
1. **Self‑mod risk:** Over‑tuning harms UX/brand.
   - Mitigation: typed proposals, risk tiers, Shadow/A‑B, approvals, rollback windows.
2. **Memory poisoning:** Bad facts persist.
   - Mitigation: evidence thresholds, provenance, TTL, contradiction sweeps, quarantine.
3. **Cost creep:** Broader search/retries inflate spend.
   - Mitigation: budgets as inputs, fast‑path caches, LFU eviction, caps/alerts.
4. **Latency regressions:** Over‑eager changes slow paths.
   - Mitigation: p95 guardrails, breadth‑then‑depth, failovers, degradations.
5. **Approval fatigue:** Too many Medium/High proposals.
   - Mitigation: tune thresholds; batch reviews; domain delegation.

---

## 17) Validation Plan (Proof)
- **Datasets:** 175 Q&A items (real/edge/adversarial) + platform journey metrics (activation funnel, to‑do add flow).
- **Policies:** min_confidence **0.90**; cite_if_confidence_below **0.95**; block_if_conflict_over **0.30**.
- **Pass:**
  - GQS **+≥15%** vs baseline; citation **≥90%**; contradictions **≤3%**.
  - At least one **p95 −35%** improvement via self‑evolution in either answers or to‑do flow.
  - Cost/journey **≤ baseline ±10%** after warm‑up.
- **Audit:** Manual spot‑check of 50 answers; publish before/after tables; attach replay bundles.

---

## 18) Glossary
- **GQS (Grounding Quality Score):** coverage × mean confidence × (1 − contradiction penalty).
- **SLO (Service Level Objective):** target for a metric (e.g., p95 ≤ 2.0s).
- **Shadow (test):** mirror workload; no user exposure.
- **A‑B (test):** control vs candidate on real traffic slices.
- **LFU (Least Frequently Used):** cache eviction policy.
- **TTL (Time To Live):** memory expiry.
- **p50/p95:** 50th/95th percentile latency.
- **Change‑Proposal:** typed, auditable change request with diff, rationale, expected deltas, risk, rollback.
- **Fast‑path:** cached plan macro for a frequent intent/journey.
- **Degradation policy:** pre‑approved reduced behavior if budgets can’t be met.
- **ODPVD‑R:** Observe → Diagnose → Propose → Validate → Deploy → Rollback.

---

## 19) Success Criteria (90‑day)
- 5 paying SMEs on Starter/Pro; 2 public case studies.
- >**90%** citation rate and **≤3%** contradictions where applicable.
- Demonstrate **self‑evolving** wins: one **p95 −35%** improvement (answers or to‑do), and one **+≥8%** activation or creation uplift.
- ≥ **1** validated Low‑risk auto‑change per active tenant per week.

---

**Final word:** Requirements are historical; **signals are living**. AMP turns those signals into safe, auditable evolution — for answers **and** for entire platforms, including the humble to‑do app that learns from its own clicks.