-- Add migration script here
CREATE TABLE task (
    "profile_id" UUID NOT NULL,
    "task_uuid" UUID,
    "task_type" VARCHAR(64),
    "state" VARCHAR(64),
    "source_file" TEXT,
    "result_file" TEXT,
    "created_at" timestamptz(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" timestamptz(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_profile FOREIGN KEY(profile_id) REFERENCES profile(id) ON DELETE CASCADE
)