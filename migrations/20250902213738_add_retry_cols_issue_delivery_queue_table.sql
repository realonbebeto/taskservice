-- Add migration script here
ALTER TABLE issue_delivery_queue 
ADD COLUMN n_retries INT NOT NULL,
ADD COLUMN execute_after INT NOT NULL