-- Add migration script here
CREATE TABLE profile_tokens(
    profile_token TEXT NOT NULL,
    profile_id UUID NOT NULL REFERENCES profile (id),
    PRIMARY KEY (profile_token)
);