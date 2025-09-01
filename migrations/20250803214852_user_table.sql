-- Add migration script here
CREATE TABLE profile (
    "id" UUID,
    "first_name" VARCHAR(64),
    "last_name" VARCHAR(64),
    "email" TEXT NOT NULL UNIQUE,
    "created_at" timestamptz(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" timestamptz(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(id)
)