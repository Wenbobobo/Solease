# Agent Guide (MVP)

## Goal
Build a Solana/Anchor program for .sol domain-backed lending:
- Pool fast-loan (USDC) with conservative caps
- P2P offers with fixed APR and duration
- Default -> auction (english bids + buy-it-now linear decay)

## Hard Constraints
- Only accept UNWRAPPED + NOT TOKENIZED .sol domains as collateral.
- Reject domains with dangerous registrar/admin configuration.
- Split flow:
  1) setup_collateral
  2) verify_and_withdraw
- Must have deterministic state machine and TDD coverage.

## Repo Structure
- programs/solname-credit (Anchor program)
- tests/ (ts integration tests)
- app/ (Next.js front-end, minimal demo)
- docs/ (PRD + architecture + threat model)

## Development Rules
1. Write tests first for each instruction.
2. Each instruction must validate invariants on-chain (do not trust the UI).
3. Keep MVP simple: fixed APR, strict caps, minimal dependencies.
4. Add events for each state transition for easier UI tracking.

## Commands
- anchor build
- anchor test
- pnpm test (front-end)
