-- Add migration script here
CREATE TYPE header_pair AS (name TEXT, value BYTEA);
CREATE TABLE idempotency (
    "profile_id" UUID NOT NULL,
    "idempotency_key" TEXT NOT NULL,
    "response_status_code" SMALLINT NOT NULL,
    "response_headers" header_pair [] NOT NULL,
    "response_body" BYTEA NOT NULL,
    "created_at" timestamptz(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" timestamptz(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(profile_id, idempotency_key),
    CONSTRAINT fk_profile_idempotency FOREIGN KEY(profile_id) REFERENCES profile(id) ON DELETE CASCADE
);