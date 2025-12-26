TITLE: QF Network — SPIN Consensus (dev context for coding agent)

WHAT SPIN IS
- SPIN = Short-term Parallel Incremental Network agreement for QF Network (QFN).
- Goal: sub-second block production (≈0.1 s cited) with “instant” UX and later cryptographic secure finality via an
anchor mechanism. Sources: IQ.wiki summary, CMC profile, and QF posts.
- Architecture pattern: two-layer consensus:
 1. FastChain: ultra-low-latency block production for user-visible finality (optimistic).
1. AnchorChain: slower BFT-style finality that periodically “anchors” FastChain and advances a SecureUpTo watermark
(highest securely finalized FastChain height).

KEY REPOS / CODE SURFACES (Substrate/Polkadot SDK)
- Monorepo: github.com/QuantumFusion-network/qf-solochain
 - consensus crates present:
 - client/consensus-spin
 - primitives/consensus-spin
 - Docs & tooling:
 - make qf-run, qf-node, qf-runtime, zombienet configs; node registers to telemetry.qfnetwork.xyz
- Secure finality client (TS):
 - github.com/QuantumFusion-network/secure-finality-tools
- packages/secure-finality: tiny Polkadot.js helpers to demo “Secure finalized” via SecureUpTo watermark; current dev
mentions sudo-driven updates for demos.

SPIN ↔ Polkadot MAPPING (mental model for contributors)
- Polkadot hybrid consensus separates block production from finality (BABE/AURA + GRANDPA). SPIN follows a similar
separation of concerns:
 - FastChain ≈ Polkadot block production side (liveness/throughput).
 - AnchorChain ≈ GRANDPA-like finality (deterministic finalization rounds).
- For light/bridge clients, Polkadot uses BEEFY to export succinct finality evidence; in SPIN terms, the “SecureUpTo”
watermark plays the role of “what height is safely finalized/anchored” for clients to trust.

WHAT TO IMPLEMENT / VERIFY (agent tasks)
1. Node/runtime side
   - Read QFN runtime pallets for the anchor/secure-finality surface (look for an “anchor” pallet or storage exposing
     SecureUpTo).
   - In consensus-spin crates, locate:
     - block import pipeline & proposer integration,
     - justification/commit emission for anchor events,
     - any session/authority set transitions relevant to finality.
   - Ensure Zombienet scenarios include FastChain churn (reorg windows) and AnchorChain advancement; export metrics for:
     - best, finalized, secure_up_to heights
     - time-to-anchor (Fast→Anchor lag)
2. TypeScript client side (Polkadot.js)
   - Consume SecureUpTo watermark from chain state/events (or via secure-finality-tools package when available).
   - Classify user confirmations:
     - "instant": N FastChain blocks (optimistic; reorg-risk capped by anchor cadence).
     - "secure": height ≤ SecureUpTo (anchored; safe).
   - Add a guard: reject proofs older than last authority-set change (mirrors GRANDPA practice).
3. Testing (Zombienet)
   - Spin (pun intended) local net: 4–7 validators; tighten block times to target sub-second; inject adversarial timing
     to force short reorgs on FastChain.
   - Periodically advance anchor; assert SecureUpTo increases monotonically; verify clients stop accepting states >
     SecureUpTo as “secure.”
4. Dev ergonomics
   - Makefile shortcuts (qf-run, qf-test, qf-chainspec) and Docker flow for reproducible CI.
   - Telemetry labels to distinguish fast vs anchor consensus progression.
SECURITY/SAFETY NOTES
- Treat FastChain finality as probabilistic; never bridge or pay out critical assets off “instant” unless policy
explicitly allows risk.
- Watermark updates must be authenticated by anchor authorities (like GRANDPA justifications/authority-set guarded
changes). Clients should pin to the last known good SecureUpTo if updates stall.

CHECKLIST FOR THE AGENT
- [ ] Parse repo tree for `client/consensus-spin` & `primitives/consensus-spin`; index public traits/structs.
- [ ] Locate runtime pallet that exposes `SecureUpTo` (storage item and events).
- [ ] Add a Polkadot.js helper: `await api.query.anchor.secureUpTo()` (or equivalent) with retries and chain metadata
guards.
- [ ] Build local node: `make qf-run`; confirm telemetry shows node.
- [ ] Write zombienet test: fast blocks, periodic anchor, assert client confirmation policy.
- [ ] If bridging: model Polkadot BEEFY-like proof flow; don’t equate “instant” with irreversible.

GOTCHAS / EXPECTATIONS
- Some demos currently update anchor via sudo (dev-mode). Production must replace with real finality gadget election and
signatures.
- Substrate GRANDPA docs are your reference for anchoring/finality plumbing (traits, justifications, authority-set
changes).

REFERENCE LINKS (for agent to cite/anchor reasoning)
- QF solo chain repo (paths show consensus-spin crates & build commands).
- Secure-up-to client & demo (SecureUpTo watermark; TS helpers).
- Public overviews of SPIN & perf targets (0.1s blocks, dual-layer finality).
- Polkadot consensus/finality docs for design mapping (hybrid consensus, GRANDPA, light clients, BEEFY).
