-- Add migration script here
CREATE TABLE issue_delivery_queue (
    "task_issue_id" UUID NOT NULL,
    "profile_email" TEXT NOT NULL,
    PRIMARY KEY (task_issue_id, profile_email),
    CONSTRAINT fk_profile_issue FOREIGN KEY(task_issue_id) REFERENCES task(id) ON DELETE CASCADE
);