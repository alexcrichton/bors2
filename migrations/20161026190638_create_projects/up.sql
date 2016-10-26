CREATE TABLE projects (
  id SERIAL PRIMARY KEY,
  repo_user VARCHAR NOT NULL,
  repo_name VARCHAR NOT NULL,
  github_webhook_secret VARCHAR NOT NULL,
  travis_webhook_secret VARCHAR NOT NULL,
  appveyor_webhook_secret VARCHAR NOT NULL
);
