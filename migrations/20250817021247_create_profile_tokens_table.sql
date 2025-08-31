-- Add migration script here
CREATE TABLE profile_tokens(
    "profile_token" TEXT NOT NULL,
    "profile_id" UUID NOT NULL,
    PRIMARY KEY (profile_token),
    CONSTRAINT fk_profile_token FOREIGN KEY(profile_id) REFERENCES profile(id) ON DELETE CASCADE
);