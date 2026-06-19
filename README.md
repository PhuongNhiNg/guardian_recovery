# guardian_recovery

## Project Title
guardian_recovery

## Project Description
Losing the secret key of a Stellar account usually means losing the account forever. `guardian_recovery` is a Soroban smart contract that turns account recovery into a social, M-of-N process: the account owner pre-registers a set of trusted guardians (family members, friends, hardware wallets, other devices) and an approval threshold. If the owner loses access, a candidate new owner can start a recovery; once enough guardians approve, the candidate becomes the new authoritative owner. The original owner can cancel any pending recovery at any time while they still hold their key, so honest owners always retain an exit. No real XLM is moved by the contract; the on-chain state transition itself is the deliverable.

## Project Vision
Make self-custody on Stellar safe for non-technical users. Today, losing a seed phrase is unrecoverable for most retail users, which is why many of them never move off centralized exchanges in the first place. By giving every Stellar account a built-in, programmable safety net, `guardian_recovery` aims to make "I lost my key" a recoverable event rather than a catastrophic one, without giving custodians control over the funds.

## Key Features
- M-of-N guardian social recovery with a configurable threshold and guardian set, stored per-owner in contract instance storage.
- Three-step recovery flow: `initiate_recovery` (candidate starts), `approve_recovery` (each guardian signs once), `execute_recovery` (state transition once the threshold is met).
- Owner override at any time via `cancel_recovery`, so a malicious or mistaken recovery can be killed by the real owner.
- `set_guardians` lets the owner rotate the guardian set and threshold (e.g. remove a compromised device, add a new one).
- Read-only views `get_approvals`, `get_guardians`, and `get_threshold` make it trivial to build a status UI on top of the contract.
- Explicit `#[contracterror]` enum gives clear, typed failure modes (`NotAGuardian`, `ThresholdNotReached`, `AlreadyApproved`, `RecoveryAlreadyExecuted`, etc.) instead of opaque panics.
- Authorisation is enforced with `require_auth()` on every state-mutating call, so only the owner can configure guardians, only the candidate can initiate, only registered guardians can approve, and only the owner can cancel.

## Contract

- **Network:** Stellar Testnet (Public)
- **Scope:** identity dApp — see `contracts/guardian_recovery/src/lib.rs` for the full guardian_recovery business logic.
- **Functions exposed:** see `Key Features` above and the `pub fn` list in `lib.rs`.
- **Contract ID:** `CAGMSC5FWPD427A4PESE5BPMETNM6AO5ONKEETHUQDCYFJNC5TZC3S3L`
- **Explorer template:** `https://stellar.expert/explorer/testnet/tx/55b9eaabca5c364c554082a73cc0bf5006602ffc5239ce0e71412e365d57577b`


## Future Scope
- Time-locked recovery: require a configurable delay (e.g. 24–72 h) between `initiate_recovery` and `execute_recovery` so the original owner has a guaranteed window to cancel a malicious attempt.
- Per-guardian weights instead of a flat M-of-N count, supporting setups where some guardians (e.g. a hardware key) carry more weight than others.
- Owner-initiated recovery that does not require the candidate to be known up front (commit–reveal of the new public key by the approvers).
- Optional recovery guardian fees / bounties, so off-chain relayers have an incentive to finalise recoveries.
- A reference frontend (Freighter + a small React app) that walks a real user through the full `set_guardians` -> `initiate_recovery` -> `approve_recovery` -> `execute_recovery` flow.
- A `replace_guardian` helper that atomically swaps one guardian in a single transaction, reducing the attack surface of full `set_guardians` rotations.
- Full unit test suite (`#[test]` module) covering happy path, double-approval rejection, non-guardian rejection, threshold-not-reached, and owner-cancel beats execute.

## Profile

- **Name:** <!-- Fill github name -->
- **Project:** `guardian_recovery` (identity)
- **Built with:** Soroban SDK 25, Rust, Stellar Testnet
