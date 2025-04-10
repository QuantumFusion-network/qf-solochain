# SPIN Secondary Leader election

## Title: SPIN Secondary Leader election algorithm

## Date
2025-04-10
Last status update: 2025-04-10

## Status
- [x] Proposed
- [ ] Accepted
- [ ] Rejected
- [ ] Deprecated
- [ ] Superseded by ADR #X

### Implementation Status
- [x] Planned
- [ ] In Development
- [ ] Implemented
- [ ] Verified
- [ ] Discontinued

## People
- **Decision-makers**: 
- **Consulted**: 
- **Informed**: 
  
*Note: People listed in "Informed" should submit a PR to check their name after reading this ADR. This can be done during the initial review process of the ADR upload/commit PR (when the file is first uploaded to GitHub and the author requests reviews), or in a separate PR after the ADR is merged.*

## Context

The core idea is to allow a designated secondary author to produce a block for a slot if the primary author fails to do so within a reasonable timeframe. Verification must be possible using only the block header and the parent state.

## Problem (Optional)

Nodes that are elected as a leader for a given slot must be able to produce a block for that slot. If the primary leader fails to produce a block, the secondary leader should be able to step in and produce a block.

## Decision

Assumptions:

- We want minimal changes to the core SPIN structure (pre-digest still contains the target slot).
- The mechanism should be deterministic. Given a slot number and the authority set, everyone must agree on who the primary and potential secondary authors are.
- We need to modify both the block production (proposer) logic and the block verification logic.
- The runtime might need awareness to handle potential penalties for missed slots (via Offences or removing authority from the authority set).
- The secondary author should be able to produce a block for the slot if the primary author fails to do so.

**Proposed Mechanism:**

1.  **Deterministic Secondary Author:** For any given slot `S`, we already have the primary author `P_primary` determined by `slot_author`. We need to define a deterministic way to find the *secondary* author, `P_secondary`, for that same slot `S`. A simple and common approach is to pick the *next* authority in the current session's round-robin order.

    *   Modify `slot_author` or add a new function, e.g., `get_slot_authors(slot, session_length, authorities)` that returns `Option<(PrimaryAuthorityId, SecondaryAuthorityId)>`.
    *   The secondary could be `authorities[(primary_index + 1) % authorities.len()]`. Handle edge cases like only one authority.

2.  **Block Production Logic Modification (Proposer):**
    *   When a node is scheduled to produce a block for slot `S`:
        *   Calculate `(P_primary, P_secondary)` for `S`.
        *   **If the node's key matches `P_primary`:** Propose the block as soon as slot `S` begins (or according to existing logic).
        *   **If the node's key matches `P_secondary`:** Do *not* propose immediately. Wait for a predefined portion of the slot duration (e.g., `SlotDuration / 2` or `SlotDuration * 2 / 3`). Let's call this `T_secondary_wait`.
        *   If, after `T_secondary_wait`, the node hasn't seen a valid block proposed for slot `S` by `P_primary`, then `P_secondary` proceeds to claim the slot (using its *own* key) and propose a block. The pre-digest *still contains slot `S`*. The seal will contain the signature from `P_secondary`.
    *   **Handling `N` consecutive misses:** This mechanism inherently handles consecutive misses. If `P_primary` for slot `S` misses, `P_secondary(S)` produces. If `P_primary` for slot `S+1` (who might be `P_secondary(S)`) also misses its primary slot, then `P_secondary(S+1)` (the next distinct authority in line) gets a chance after its `T_secondary_wait`. The "N" becomes more of a parameter for the runtime's slashing/offence mechanism rather than the consensus proposal logic itself.

3.  **Block Verification Logic Modification (`check_header_slot_and_seal`):**
    *   This function needs to accept blocks signed by *either* the primary or the secondary author for the given slot.
    *   **Steps:**
        1.  Extract the `slot` from the pre-digest (as before).
        2.  Extract the `seal` digest item and the signature `sig` it contains. Determine the public key `P_signer` corresponding to the signature (this might require changes if the public key isn't easily recoverable from the signature itself, potentially requiring the public key to be included in the seal digest or fetched based on the signature). *Correction*: The `seal` function uses the *public key* to find the signing key in the keystore. Verification typically involves the signature, the message (pre-hash), and the *expected* public key.
        3.  Calculate the *expected* primary author `P_primary` for the `slot` using `slot_author`.
        4.  Calculate the *expected* secondary author `P_secondary` for the `slot` (using the same deterministic rule as the proposer, e.g., `authorities[(primary_index + 1) % authorities.len()]`).
        5.  Check 1: Verify the signature `sig` against the `P_primary` public key and the pre-hash of the header.
        6.  Check 2: Verify the signature `sig` against the `P_secondary` public key and the pre-hash of the header.
        7.  **Result:** If *either* Check 1 *or* Check 2 succeeds, the seal is considered valid with respect to the slot author rules. If both fail, return `BadSignature`.
        8.  The rest of the function (`Deferred` check, etc.) remains the same.

4.  **Runtime Awareness (Optional but Recommended):**
    *   The runtime (e.g., in `on_initialize` or via an inherent processing block authorship) should be made aware of who actually signed the block.
    *   It can compare the actual signer (`P_signer`, derived from the seal verification or passed via inherent) with the expected primary author `P_primary` for the slot (derived from the pre-digest).
    *   If `P_signer == P_secondary`, the runtime knows `P_primary` missed the slot. It can then trigger its offence reporting logic (e.g., using `sp_staking::offence::ReportOffence`) against `P_primary`. This is where the concept of "N missed slots" can be implemented as a threshold for reporting/slashing.

**Implementation Steps (Conceptual):**

1.  **Define Secondary Author Logic:**
    *   Create `fn get_secondary_author<P: Pair>(primary_index: usize, authorities: &[AuthorityId<P>]) -> Option<&AuthorityId<P>>`.
    *   Ensure it handles wrapping and the case of a single authority gracefully (where there might be no secondary).

2.  **Modify `check_header_slot_and_seal`:**
    *   Inside the function, after getting `slot` and `seal`/`sig`:
    *   Calculate `primary_author` using `slot_author`. Get its index if needed.
    *   Calculate `secondary_author` using `get_secondary_author`.
    *   Perform the verification check:
        ```rust
        let pre_hash = header.hash();
        let primary_valid = P::verify(&sig, pre_hash.as_ref(), primary_author);

        let secondary_valid = if let Some(sec_author) = secondary_author {
            P::verify(&sig, pre_hash.as_ref(), sec_author)
        } else {
            false
        };

        if primary_valid || secondary_valid {
            // Signature is valid under either the primary or secondary expected author
            Ok((header, slot, seal))
        } else {
            // Signature is not valid under either expected author
            Err(SealVerificationError::BadSignature)
        }
        ```
    *   Make sure `slot_author` correctly handles session boundaries and returns the index along with the author ID if needed for the secondary calculation.

3.  **Modify Proposer (`worker.rs` or similar):**
    *   In the main loop checking slots:
    *   When considering proposing for `slot`:
        *   `let primary_author = slot_author(slot, ...);`
        *   `let primary_index = authorities.iter().position(|a| a == primary_author).unwrap();`
        *   `let secondary_author = get_secondary_author(primary_index, authorities);`
        *   `let local_keys = keystore.keys(...); // Get local authority keys`
        *   `let i_am_primary = local_keys.contains(primary_author);`
        *   `let i_am_secondary = secondary_author.map_or(false, |sa| local_keys.contains(sa));`

        *   If `i_am_primary`:
            *   `claim_slot` and propose immediately.
        *   Else if `i_am_secondary`:
            *   Calculate `deadline = slot_start_time + T_secondary_wait`.
            *   Wait until `deadline`.
            *   Check if a block for `slot` has been imported/seen.
            *   If not seen: `claim_slot` (using the *secondary* key) and propose.

4.  **Runtime:**
    *   Add logic (likely in pallet implementing `frame_support::traits::OnInitialize`) to:
        *   Read the current slot from the digest.
        *   Determine `P_primary` and `P_secondary`.
        *   Determine the actual author `P_signer` (this might need an inherent data provider in consensus passing the signer's public key, or potentially reading it from the seal digest if possible/reliable).
        *   If `P_signer == P_secondary`, report offence for `P_primary`. Keep track of consecutive offences if needed for the "N" threshold.



### Decision Drivers (Optional)


### Alternatives Considered

#### Alternative 1: Anyone Can Produce (After Timeout)

*   **Mechanism:** If the primary author for slot `S` doesn't produce a block within `T_wait`, *any* active authority can attempt to produce and sign a block for slot `S`. The first valid block seen by the network for slot `S` is accepted.
*   **Pros:**
    *   Maximizes chance of *some* block being produced if primary fails.
    *   More fair and proof-of-work-like
    *   Relatively simple verification rule (signature must match *any* known authority from the set).
*   **Cons:**
    *   Increased network traffic from potentially competing blocks, which is counter intuitive to the goal of SPIN.
    *   High potential for contention: Multiple authorities might race to produce, leading to temporary micro-forks and wasted resources.
    *   Requires careful timeout tuning (`T_wait`).

#### Alternative 2. Deterministic Random Backup

*   **Mechanism:** Similar to Primary/Secondary, but instead of the *next* authority in sequence, a pseudo-random function (seeded by slot number, epoch randomness, etc.) deterministically selects a *different* authority from the set as the backup.
*   **Pros:**
    *   Distributes backup load more randomly than strictly sequential. Might be fairer if certain authorities are frequently adjacent to unreliable ones.
*   **Cons:**
    *   Counterintuitive to the SPIN design: adds more computation time and complexity.
    *   More complex calculation: Requires a shared, deterministic pseudo-random source available to consensus.
    *   No significant liveness or simplicity benefit over sequential secondary. Adds complexity for marginal gain.
    *   Proposal/Verification logic complexity is similar to Primary/Secondary, just with a different way to pick the backup.

### Consequences (Optional)
- **Positive**: Increases network liveness and fault tolerance by allowing a secondary author to step in if the primary fails.
- **Negative**: Could potentially impact block time negatively if there are many failing primary authors. Requires maintaining stable, reliable own nodes.
- **Risks**: 100 ms block time lag

## Implementation Notes (Optional)

## Confirmation (Optional)

1.  **Deterministic Secondary:** Given a slot number and the active authority set, all nodes MUST deterministically calculate the same primary and secondary author for that slot.
2.  **Primary Author Success:** If the primary author produces a valid block for their designated slot within the expected timeframe, it MUST be accepted by the network.
3.  **Secondary Author Trigger:** A node designated as the secondary author for a slot MUST only attempt to produce a block *after* a defined timeout within the slot has passed *without* observing a valid block from the primary author.
4.  **Secondary Author Production:** If the primary author fails to produce a block by the timeout, the designated secondary author MUST be able to produce and sign a valid block for that same slot using their own key.
5.  **Secondary Block Acceptance:** Blocks validly signed by the *correct* secondary author for a given slot (after the primary's timeout) MUST be accepted by the network and allow the chain to progress.
6.  **Invalid Author Rejection:** Blocks signed by an authority who is neither the designated primary nor the designated secondary for that slot MUST be rejected.
7.  **(Optional) Offence Reporting:** The runtime SHOULD be able to identify when a block was produced by a secondary author and correctly attribute the missed slot offence to the primary author.

## References

[AURA](https://github.com/paritytech/polkadot-sdk/tree/master/substrate/frame/aura)
[Offences](https://github.com/paritytech/polkadot-sdk/tree/master/substrate/frame/offences)
