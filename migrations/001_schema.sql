CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE accounts (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    owner      TEXT        NOT NULL,
    balance    BIGINT      NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT balance_non_negative CHECK (balance >= 0)
);

CREATE TABLE ledger_entries (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id  UUID        NOT NULL REFERENCES accounts(id),
    amount      BIGINT      NOT NULL,
    transfer_id UUID        NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX ledger_entries_account_id_idx ON ledger_entries(account_id);
CREATE INDEX ledger_entries_transfer_id_idx ON ledger_entries(transfer_id);

CREATE TABLE idempotency_keys (
    key        TEXT        PRIMARY KEY,
    response   JSONB       NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);