# stellar-ai-payments-contracts

[![CI](https://github.com/Devcyprian/stellar-ai-payments-contracts/actions/workflows/ci.yml/badge.svg)](https://github.com/Devcyprian/stellar-ai-payments-contracts/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Stellar Wave](https://img.shields.io/badge/Stellar-Wave%205-7b2d8b)](https://communityfund.stellar.org)
[![Soroban](https://img.shields.io/badge/Soroban-Protocol%2021-blue)](https://developers.stellar.org/docs/build/smart-contracts)

> **Production-ready Rust/Soroban smart contracts for AI agent payment infrastructure on Stellar — escrow, fee sponsorship vault, and multi-destination payment routing.**

---

## Why This Matters for the Stellar Ecosystem

Soroban smart contracts are Stellar's most powerful new capability, yet there is a critical gap: no open-source, auditable contract suite exists specifically for AI agent payment flows. This repository fills that gap with three composable contracts:

1. **AgentEscrow** — Trustless escrow with TTL expiry, enabling agents to lock funds for a service and release or refund atomically
2. **SponsoredFeeVault** — On-chain fee sponsorship pool so AI agents can transact with zero XLM balance, removing the biggest onboarding barrier for agent developers
3. **PaymentRouter** — Basis-point routing table for splitting a single payment across multiple recipients atomically — essential for revenue sharing, royalties, and multi-agent coordination

Together, these contracts form the on-chain backbone of the `stellar-ai-payments` ecosystem and are designed to be deployed, forked, and extended by the Stellar developer community.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                  AI Agent / SDK / dApp                          │
│           (@stellar-ai-payments/sdk calls these)                │
└──────────────────────┬──────────────────────────────────────────┘
                       │  Soroban RPC
       ┌───────────────┼───────────────────┐
       │               │                   │
┌──────▼──────┐ ┌──────▼──────────┐ ┌─────▼──────────┐
│ AgentEscrow │ │SponsoredFeeVault│ │ PaymentRouter  │
│             │ │                 │ │                │
│ deposit()   │ │ deposit()       │ │ set_routes()   │
│ release()   │ │ register_agent()│ │ route()        │
│ refund()    │ │ deduct_fee()    │ │ get_routes()   │
│ get_payment │ │ withdraw()      │ │ route_count()  │
│ get_admin() │ │ get_balance()   │ │ get_admin()    │
└──────┬──────┘ └──────┬──────────┘ └─────┬──────────┘
       └───────────────┴───────────────────┘
                       │
              ┌────────▼────────┐
              │  shared/ crate  │
              │  PaymentRecord  │
              │  PaymentStatus  │
              │  Error codes    │
              └─────────────────┘
```

---

## Repository Structure

```
stellar-ai-payments-contracts/
├── contracts/
│   ├── agent-escrow/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs          # Escrow: deposit, release, refund, TTL, 6 tests
│   ├── sponsored-fee-vault/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs          # Vault: deposit, register/deregister agent, deduct, withdraw, 5 tests
│   └── payment-router/
│       ├── Cargo.toml
│       └── src/lib.rs          # Router: BPS routing table, route(), 5 tests
├── shared/
│   ├── Cargo.toml
│   └── src/lib.rs              # PaymentRecord, PaymentStatus, Error, helpers
├── .cargo/
│   └── config.toml             # Aliases: build-wasm, test-all
├── .github/
│   ├── workflows/ci.yml        # Clippy → cargo test --all → WASM build
│   ├── CODEOWNERS
│   └── ISSUE_TEMPLATE/
│       └── stellar_wave_task.md
├── Cargo.toml                  # Workspace — Soroban SDK 21.x, Protocol 21
├── rust-toolchain.toml         # Pinned to stable + wasm32-unknown-unknown
├── .env.example
├── CONTRIBUTING.md
├── SECURITY.md
├── CHANGELOG.md
└── LICENSE                     # MIT
```

---

## Contracts Reference

### AgentEscrow

Holds funds in escrow until released by the sender/admin, or automatically refunded after a configurable TTL. Designed for agent-to-service payment flows where the service must be verified before funds are released.

| Function | Auth | Description |
|----------|------|-------------|
| `initialize(admin)` | admin | Set contract admin |
| `deposit(id, sender, recipient, asset, amount, ttl)` | sender | Lock funds; creates `PaymentRecord` |
| `release(id, caller)` | sender or admin | Transfer to recipient |
| `refund(id)` | anyone | Refund to sender after TTL expires |
| `get_payment(id)` | — | Read a `PaymentRecord` |
| `get_admin()` | — | Read admin address |

**Constants:** `MIN_TTL_SECONDS = 60`, `MAX_TTL_SECONDS = 2,592,000` (30 days)

**Error codes:** `NotFound(1)`, `Unauthorized(2)`, `AlreadyExists(3)`, `Expired(4)`, `InsufficientFunds(5)`, `InvalidAmount(6)`

**Tests (6):** deposit+release, invalid amount, duplicate deposit, refund after expiry, refund before expiry (fails), get nonexistent payment

---

### SponsoredFeeVault

An on-chain XLM pool that covers transaction fees for registered AI agents. Admins deposit XLM, register agents with per-tx allowances, and agents deduct fees without holding any XLM themselves.

| Function | Auth | Description |
|----------|------|-------------|
| `initialize(admin)` | admin | Set admin |
| `deposit(from, asset, amount)` | from | Add XLM to vault |
| `register_agent(caller, agent, allowance)` | admin | Allowlist agent with max fee per tx |
| `deregister_agent(caller, agent)` | admin | Remove agent from allowlist |
| `deduct_fee(agent, asset, fee)` | agent | Deduct fee (≤ allowance) from vault |
| `withdraw(caller, asset, amount)` | admin | Withdraw XLM from vault |
| `get_balance()` | — | Current vault balance |
| `get_admin()` | — | Read admin address |

**Constant:** `MIN_DEPOSIT = 1,000,000 stroops` (0.1 XLM)

**Tests (5):** deposit increases balance, invalid deposit amount, register+deduct, unregistered agent blocked, deduct exceeds allowance

---

### PaymentRouter

Routes a single incoming payment to multiple destinations using basis-point (BPS) splits. BPS values must sum to exactly 10,000 (100%). Enables revenue sharing, royalty distribution, and multi-agent coordination in one atomic transaction.

| Function | Auth | Description |
|----------|------|-------------|
| `initialize(admin)` | admin | Set admin |
| `set_routes(caller, routes)` | admin | Set routing table (BPS must sum to 10,000) |
| `clear_routes(caller)` | admin | Remove all routes |
| `route(sender, asset, amount)` | sender | Pull full amount, split to all destinations |
| `get_routes()` | — | Read current routing table |
| `route_count()` | — | Number of configured routes |
| `get_admin()` | — | Read admin address |

**Constant:** `TOTAL_BPS = 10,000`

**Tests (5):** set+get routes, BPS must sum to 10000, route splits correctly, non-admin blocked, invalid amount

---

### Shared Types (`shared/`)

Common types used across all three contracts. `#![no_std]` — no heap allocations outside Soroban SDK types.

```rust
pub struct PaymentRecord {
    pub id: String,
    pub sender: Address,
    pub recipient: Address,
    pub amount: i128,
    pub asset: Address,
    pub status: PaymentStatus,  // Pending | Released | Refunded | Expired
    pub created_at: u64,
    pub expires_at: u64,
}

pub enum Error {
    NotFound = 1, Unauthorized = 2, AlreadyExists = 3,
    Expired = 4, InsufficientFunds = 5, InvalidAmount = 6,
}
```

---

## Setup & Development

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown
```

### Run Tests

```bash
cargo test --all
# or using the alias:
cargo test-all
```

### Build WASM

```bash
cargo build --release --target wasm32-unknown-unknown \
  -p agent-escrow -p sponsored-fee-vault -p payment-router

# or using the alias:
cargo build-wasm -p agent-escrow
```

### Deploy to Testnet

```bash
cp .env.example .env
# Fill in ADMIN_SECRET_KEY

stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/agent_escrow.wasm \
  --source $ADMIN_SECRET_KEY \
  --network testnet
```

---

## Stellar Ecosystem Alignment

| Feature | SDF Reference | This Repo |
|---------|--------------|-----------|
| Soroban smart contracts | [Soroban Docs](https://developers.stellar.org/docs/build/smart-contracts) | All 3 contracts |
| Fee sponsorship | [Fee Bump Transactions](https://developers.stellar.org/docs/learn/encyclopedia/transactions-specialized/fee-bump-transactions) | SponsoredFeeVault |
| Agentic payments | [Agentic Payments](https://developers.stellar.org/docs/build/agentic-payments) | AgentEscrow + PaymentRouter |
| x402 protocol | [x402 on Stellar](https://stellar.org/x402) | Complements SDK x402 client |
| MPP | [MPP on Stellar](https://developers.stellar.org/docs/build/agentic-payments/mpp) | PaymentRouter (on-chain MPP) |

---

## Related Packages

| Package | Description |
|---------|-------------|
| [`stellar-ai-payments-sdk`](https://github.com/Devcyprian/stellar-ai-payments-sdk) | TypeScript SDK that calls these contracts |
| [`stellar-ai-payments-adapters`](https://github.com/Devcyprian/stellar-ai-payments-adapters) | LangChain, OpenAI, Claude, Express adapters |
| [`stellar-ai-payments-docs`](https://github.com/Devcyprian/stellar-ai-payments-docs) | Full documentation site |

---

## Contributing

This project participates in **Stellar Wave 5** via [Drips Wave](https://drips.network/wave). Issues labeled `wave5` are open for community contributors.

See [CONTRIBUTING.md](CONTRIBUTING.md) for branch model, commit format, and PR checklist.

**Branch model:** all PRs target `develop` → squash merge → `main` for releases.

---

## License

MIT © 2026 [Devcyprian](https://github.com/Devcyprian)
