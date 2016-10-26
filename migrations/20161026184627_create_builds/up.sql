CREATE TABLE builds (
  id SERIAL PRIMARY KEY,
  pull_request_id INTEGER NOT NULL,
  head_commit VARCHAR NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT now(),
  status INTEGER NOT NULL
);

ALTER TABLE builds ADD CONSTRAINT fk_builds_pull_request_id
  FOREIGN KEY (pull_request_id) REFERENCES pull_requests (id);
