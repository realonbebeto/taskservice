-- Add migration script here
CREATE TYPE task_state AS ENUM (
    'notstarted',
    'inprogress',
    'completed',
    'paused',
    'failed'
);