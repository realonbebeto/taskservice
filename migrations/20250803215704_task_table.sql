-- Add migration script here
CREATE TABLE task (
    "profile_id" VARCHAR(64) NOT NULL,
    "task_uuid" VARCHAR(64),
    "task_type" VARCHAR(64),
    "state" VARCHAR(64),
    "source_file" TEXT,
    "result_file" TEXT,
    "created_at" timestamptz(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" timestamptz(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    constraint fk_profile foreign key(profile_id) references profile(id)
)