-- Add migration script here
ALTER TABLE issue_delivery_queue
ADD COLUMN n_retries INT NOT NULL,
    ADD COLUMN last_attempt timestamptz(3),
    ADD COLUMN created_at timestamptz(3) NOT NULL DEFAULT CURRENT_TIMESTAMP