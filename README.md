![CI](https://github.com/Devcyprian/stellar-ai-payments-contracts/actions/workflows/ci.yml/badge.svg)

# stellar-ai-payments-contracts

Rust/Soroban smart contracts for AI agent payments on Stellar.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  AI Agent / SDK                     │
├──────────────┬──────────────────┬───────────────────┤
│ AgentEscrow  │ SponsoredFeeVault│  PaymentRouter    │
│  Deposit /   │  Vault for fee   │  BPS-based split  │
│  Release /   │  sponsorship     │  routing          │
│  Refund      │                  │                   │
├──────────────┴──────────────────┴───────────────────┤
│              shared/ — common types & errors        │
│              Soroban SDK 21.x                       │
└─────────────────────────────────────────────────────┘
```

## File Tree

```
contracts/
  agent-escrow/src/lib.rs       Escrow: deposit, release, refund
  sponsored-fee-vault/src/lib.rs Vault: deposit, register_agent, deduct_fee
  payment-router/src/lib.rs     Router: set_routes (BPS), route payment
shared/src/lib.rs               PaymentRecord, PaymentStatus, Error types
Cargo.toml                      Workspace
```

## Setup

```bash
# Install Rust + wasm target
rustup target add wasm32-unknown-unknown

# Run all tests
cargo test --all

# Build WASM
cargo build --release --target wasm32-unknown-unknown -p agent-escrow
```

## Contracts

### AgentEscrow
Holds funds in escrow until released by sender/admin or refunded after TTL.

### SponsoredFeeVault
Stores XLM for fee sponsorship. Admin registers agents with per-tx allowances.

### PaymentRouter
Routes a single payment to multiple destinations using basis-point splits (must sum to 10000).

## Deploy to Testnet

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/agent_escrow.wasm \
  --source ADMIN_SECRET \
  --network testnet
```

## Contract Addresses (Testnet)

| Contract | Address |
|----------|---------|
| AgentEscrow | TBD after deploy |
| SponsoredFeeVault | TBD after deploy |
| PaymentRouter | TBD after deploy |

## Testing

```bash
cargo test --all -- --nocapture
```

Each contract has 5+ tests covering happy path and error cases.
