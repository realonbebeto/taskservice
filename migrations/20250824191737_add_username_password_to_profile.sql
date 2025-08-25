-- Add migration script here
ALTER TABLE profile
ADD COLUMN username TEXT NOT NULL UNIQUE,
    ADD COLUMN password TEXT NOT NULL;