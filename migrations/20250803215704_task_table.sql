-- Add migration script here
CREATE TYPE task_state AS ENUM (
    'notstarted',
    'inprogress',
    'completed',
    'paused',
    'failed'
);
CREATE TABLE task (
    "reporter_id" UUID NOT NULL,
    "id" UUID,
    "task_type" VARCHAR(64),
    "state" task_state,
    "source_file" TEXT,
    "result_file" TEXT,
    "created_at" timestamptz(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" timestamptz(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    CONSTRAINT fk_profile FOREIGN KEY(reporter_id) REFERENCES profile(id) ON DELETE CASCADE
)