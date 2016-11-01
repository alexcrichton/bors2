use pg::GenericConnection;
use pg::rows::Row;

use errors::*;

pub struct Project {
    id: i32,
    repo_user: String,
    repo_name: String,
    github_access_token: String,
    github_webhook_secret: String,
    appveyor_token: Option<String>,
    travis_access_token: Option<String>,
}

impl Project {
    pub fn insert(conn: &GenericConnection,
                  repo_user: &str,
                  repo_name: &str,
                  github_access_token: &str,
                  github_webhook_secret: &str) -> BorsResult<Project> {
        let stmt = try!(conn.prepare("INSERT INTO projects
                                      (repo_user, repo_name, github_access_token,
                                       github_webhook_secret)
                                      VALUES ($1, $2, $3, $4)
                                      RETURNING *"));
        let rows = try!(stmt.query(&[&repo_user,
                                     &repo_name,
                                     &github_access_token,
                                     &github_webhook_secret]));
        Ok(Project::from_row(&rows.iter().next().unwrap()))
    }

    pub fn from_row(row: &Row) -> Project {
        Project {
            id: row.get("id"),
            repo_user: row.get("repo_user"),
            repo_name: row.get("repo_name"),
            github_access_token: row.get("github_access_token"),
            github_webhook_secret: row.get("github_webhook_secret"),
            appveyor_token: row.get("appveyor_token"),
            travis_access_token: row.get("travis_access_token"),
        }
    }
}
