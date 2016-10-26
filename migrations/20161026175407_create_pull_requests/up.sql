CREATE TABLE pull_requests (
  id SERIAL PRIMARY KEY,
  number INTEGER NOT NULL,
  github_id INTEGER NOT NULL,
  status INTEGER NOT NULL,
  head_ref VARCHAR NOT NULL,
  head_commit VARCHAR NOT NULL,
  title VARCHAR NOT NULL,
  approved_by VARCHAR,
  mergeable BOOLEAN NOT NULL,
  assignee VARCHAR,
  priority INTEGER NOT NULL,
  rollup BOOLEAN NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT now()
)
