CREATE TABLE projects (
  id SERIAL PRIMARY KEY,
  repo_user VARCHAR NOT NULL,
  repo_name VARCHAR NOT NULL,
  github_webhook_secret VARCHAR NOT NULL,
  github_access_token VARCHAR NOT NULL,
  travis_access_token VARCHAR,
  appveyor_token VARCHAR
);
