# idempotent-ledger-rs

A production-grade idempotent ledger API built in Rust. Demonstrates safe concurrent money transfers with correctness guarantees backed by PostgreSQL.

## What is idempotency and why does it matter?

In distributed systems, networks are unreliable. A client sending a payment request may never receive a response — not because the transfer failed, but because the response was lost. If the client retries, a naive server would process the payment twice.

An **idempotent API** guarantees that submitting the same request multiple times has the same effect as submitting it once. The client attaches an `idempotency_key` to every request — a unique string they generate. The server stores the result against that key. On retry, it returns the cached result without re-processing.

This project implements that guarantee correctly under concurrent load.

---

## Architecture

```mermaid
graph TD
    A["Client
    curl
    /app"] -->|HTTP| B["Axum HTTP Server 
    POST /transfers
    GET /transfers/:id
    GET /accounts/:id/balance"]
    B --> C["LedgerService\nbusiness logic"]
    C --> D["Db\ndata access"]
    D --> E[("PostgreSQL\naccounts · ledger_entries · idempotency_keys")]
```

---


## How a Transfer Works

### Happy path — new idempotency key

```mermaid
sequenceDiagram
    participant Client
    participant Handler
    participant LedgerService
    participant PostgreSQL

    Client->>Handler: POST /transfers
    Handler->>LedgerService: transfer()
    LedgerService->>PostgreSQL: BEGIN
    LedgerService->>PostgreSQL: SELECT FOR UPDATE (lock_accounts)
    LedgerService->>PostgreSQL: get_cached_result
    PostgreSQL-->>LedgerService: None
    LedgerService->>PostgreSQL: apply_entry debit
    LedgerService->>PostgreSQL: apply_entry credit
    LedgerService->>PostgreSQL: cache_result
    LedgerService->>PostgreSQL: COMMIT
    LedgerService-->>Handler: TransferResult
    Handler-->>Client: 200 OK
```

### Idempotent replay — same key sent again

```mermaid
sequenceDiagram
    participant Client
    participant Handler
    participant LedgerService
    participant PostgreSQL

    Client->>Handler: POST /transfers (same key)
    Handler->>LedgerService: transfer()
    LedgerService->>PostgreSQL: BEGIN
    LedgerService->>PostgreSQL: SELECT FOR UPDATE (WAITS for in-flight transfer)
    LedgerService->>PostgreSQL: get_cached_result
    PostgreSQL-->>LedgerService: TransferResult (found!)
    LedgerService->>PostgreSQL: COMMIT
    LedgerService-->>Handler: TransferResult
    Handler-->>Client: 200 OK (cached, no debit applied)
```

---

## Key Design Decisions

### `READ COMMITTED` + `SELECT ... FOR UPDATE`

The idempotency check happens **inside** the transaction, **after** acquiring row locks on the accounts:

```
BEGIN
  SELECT ... FOR UPDATE  ← serializes concurrent requests on the same accounts
  SELECT idempotency_key ← safe to check here: previous txn has committed
  if found → return cached result
  apply debit + credit
  store idempotency key
COMMIT
```

Under `READ COMMITTED` isolation, a transaction that waited for a lock sees the **latest committed data** after acquiring it. So the second concurrent request, after waiting, finds the idempotency key written by the first and returns the cached result without re-applying the transfer.

`SERIALIZABLE` isolation was considered and rejected: it takes a frozen snapshot at transaction start and aborts with error `40001` on conflict, requiring retry logic. `READ COMMITTED` + `FOR UPDATE` is simpler and more efficient.

### Consistent lock ordering prevents deadlocks

When two transfers touch the same pair of accounts in opposite directions, naive locking causes deadlocks:

```mermaid
graph LR
    subgraph "Without consistent ordering ❌"
        A1[Txn A] -->|lock Alice| B1[Alice]
        A1 -->|lock Bob| C1[Bob]
        D1[Txn B] -->|lock Bob| C1
        D1 -->|lock Alice → DEADLOCK| B1
    end

    subgraph "With consistent ordering ✅"
        A2[Txn A] -->|lock Alice| B2[Alice]
        A2 -->|lock Bob| C2[Bob]
        D2[Txn B] -->|lock Alice → WAIT| B2
    end
```

Accounts are always locked in ascending UUID order, regardless of transfer direction.

### Balance constraint enforced at the database level

```sql
CONSTRAINT balance_non_negative CHECK (balance >= 0)
```

Even if application logic has a bug, the database will reject any update that would make a balance negative. The `InsufficientFunds` error is detected by catching PostgreSQL error code `23514`.

---

## Database Schema

```mermaid
erDiagram
    accounts {
        uuid id PK
        text owner
        bigint balance
        timestamptz created_at
    }
    ledger_entries {
        uuid id PK
        uuid account_id FK
        uuid transfer_id
        bigint amount
        timestamptz created_at
    }
    idempotency_keys {
        text key PK
        jsonb response
        timestamptz created_at
    }
    accounts ||--o{ ledger_entries : "has"
```

---

## API

### `POST /transfers`

Submit a transfer. Attach an `idempotency_key` to make it safe to retry.

```bash
curl -X POST http://localhost:3000/transfers \
  -H "Content-Type: application/json" \
  -d '{
    "idempotency_key": "payment-2024-001",
    "from_account":    "a494c925-6452-4c5a-b3c8-4b52b1375ad6",
    "to_account":      "2dec16ec-4a8f-43a9-9bee-1540797fbc1b",
    "amount":          1000
  }'
```

**Response `200 OK`:**
```json
{
  "transfer_id":   "f1e2d3c4-...",
  "from_account":  "a494c925-...",
  "to_account":    "2dec16ec-...",
  "amount":        1000
}
```

`amount` is in **cents** (integer). 1000 = $10.00.

Submitting the same `idempotency_key` again returns the identical response without re-applying the transfer.

---

### `GET /transfers/:id`

Fetch a transfer by its ID.

```bash
curl http://localhost:3000/transfers/f1e2d3c4-b5a6-7890-abcd-ef1234567890
```

**Response `200 OK`:**
```json
{
  "transfer_id":   "f1e2d3c4-...",
  "from_account":  "a494c925-...",
  "to_account":    "2dec16ec-...",
  "amount":        1000
}
```

---

### `GET /accounts/:id/balance`

```bash
curl http://localhost:3000/accounts/a494c925-6452-4c5a-b3c8-4b52b1375ad6/balance
```

**Response `200 OK`:**
```json
{
  "account_id":     "a494c925-...",
  "balance_cents":  9000
}
```

---

## Running Locally

### Prerequisites
- [Docker](https://docs.docker.com/get-docker/)
- [Rust](https://rustup.rs/) (for running tests locally)

### With Docker Compose (full stack)

```bash
docker compose up --build
```

The app starts on `http://localhost:3000`. Migrations run automatically on startup.

### For local development

```bash
# 1. Start Postgres only
docker compose up -d postgres

# 2. Copy and configure environment
cp .env.example .env

# 3. Run migrations
sqlx migrate run

# 4. Run the server
cargo run
```

---

## Running Tests

The tests are integration tests that require a live Postgres instance.

```bash
# 1. Start the database
docker compose up -d postgres

# 2. Run migrations (first time only)
sqlx migrate run

# 3. Run the tests
cargo test
```

Each test truncates all tables before running, so tests are fully isolated and can be run repeatedly.

### Test coverage

| Test | What it verifies |
|---|---|
| `same_key_concurrent_produces_one_transfer` | 10 concurrent requests with the same idempotency key result in exactly one debit |
| `transfer_exceeding_balance_is_rejected` | Transfers that would make balance negative are rejected with `InsufficientFunds` |
| `sequential_transfers_maintain_balance_invariant` | The sum of all balances is conserved across transfers |

---

## Project Structure

```
src/
  main.rs      — HTTP server, route handlers
  ledger.rs    — Business logic (LedgerService)
  db.rs        — Database queries (Db)
  types.rs     — Domain types (AccountId, Money, TransferRequest, ...)
  error.rs     — LedgerError + HTTP response mapping
migrations/
  001_schema.sql
tests/
  concurrent_tests.rs
```

---

## Environment Variables

| Variable | Description | Example |
|---|---|---|
| `DATABASE_URL` | PostgreSQL connection string | `postgres://postgres:password@localhost:5050/ledger` |
| `RUST_LOG` | Log level filter | `idempotent_ledger_rs=info,tower_http=debug` |

Copy `.env.example` to `.env` to get started.

---

## Tech Stack

| | |
|---|---|
| Language | Rust |
| HTTP framework | [Axum](https://github.com/tokio-rs/axum) |
| Async runtime | [Tokio](https://tokio.rs) |
| Database | PostgreSQL 16 |
| Database client | [sqlx](https://github.com/launchbadge/sqlx) (compile-time verified queries) |
| Logging | [tracing](https://github.com/tokio-rs/tracing) (structured JSON) |
| Containerisation | Docker + docker compose |
| CI | GitHub Actions |
