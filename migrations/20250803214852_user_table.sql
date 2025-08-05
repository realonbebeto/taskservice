-- Add migration script here
CREATE TABLE profile (
    "id" VARCHAR(64) PRIMARY KEY,
    "first_name" VARCHAR(64),
    "last_name" VARCHAR(64),
    "created_at" timestamptz(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" timestamptz(3) NOT NULL DEFAULT CURRENT_TIMESTAMP
)