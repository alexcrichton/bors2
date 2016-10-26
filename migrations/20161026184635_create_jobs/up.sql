CREATE TABLE jobs (
  id SERIAL PRIMARY KEY,
  build_id INTEGER NOT NULL,
  status INTEGER NOT NULL,
  provider INTEGER NOT NULL,
  provider_account_name VARCHAR NOT NULL,
  provider_project_name VARCHAR NOT NULL,
  provider_job_version VARCHAR NOT NULL,
  provider_job_id VARCHAR NOT NULL
);

ALTER TABLE jobs ADD CONSTRAINT fk_jobs_build_id
  FOREIGN KEY (build_id) REFERENCES builds (id);

