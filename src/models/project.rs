use pg::GenericConnection;
use pg::rows::Row;

use errors::*;

pub struct Project {
    pub id: i32,
    pub repo_user: String,
    pub repo_name: String,
    pub github_repo_id: i32,
    pub github_access_token: String,
    pub github_webhook_secret: String,
    pub appveyor_token: Option<String>,
    pub travis_access_token: Option<String>,
}

impl Project {
    pub fn insert(conn: &GenericConnection,
                  repo_user: &str,
                  repo_name: &str,
                  github_repo_id: i32,
                  github_access_token: &str,
                  github_webhook_secret: &str) -> BorsResult<Project> {
        let stmt = try!(conn.prepare("INSERT INTO projects
                                      (repo_user,
                                       repo_name,
                                       github_repo_id,
                                       github_access_token,
                                       github_webhook_secret)
                                      VALUES ($1, $2, $3, $4, $5)
                                      RETURNING *"));
        let rows = try!(stmt.query(&[&repo_user,
                                     &repo_name,
                                     &github_repo_id,
                                     &github_access_token,
                                     &github_webhook_secret]));
        Ok(Project::from_row(&rows.iter().next().unwrap()))
    }

    pub fn find_by_name(conn: &GenericConnection,
                        user: &str,
                        repo: &str) -> BorsResult<Project> {
        let stmt = try!(conn.prepare("SELECT * FROM projects
                                      WHERE repo_user = $1 AND repo_name = $2
                                      LIMIT 1"));
        let rows = try!(stmt.query(&[&user, &repo]));
        match rows.into_iter().next() {
            Some(ref p) => Ok(Project::from_row(p)),
            None => Err(BorsErrorKind::MissingProject.into()),
        }
    }

    pub fn all(conn: &GenericConnection) -> BorsResult<Vec<Project>> {
        let stmt = try!(conn.prepare("SELECT * FROM projects"));
        let rows = try!(stmt.query(&[]));
        Ok(rows.iter().map(|r| Project::from_row(&r)).collect())
    }

    pub fn set_travis_token(&self,
                            conn: &GenericConnection,
                            token: &str) -> BorsResult<()> {
        let stmt = try!(conn.prepare("UPDATE projects
                                         SET travis_access_token = $1
                                       WHERE id = $2"));
        try!(stmt.query(&[&token, &self.id]));
        Ok(())
    }

    pub fn set_appveyor_token(&self,
                              conn: &GenericConnection,
                              token: &str) -> BorsResult<()> {
        let stmt = try!(conn.prepare("UPDATE projects
                                         SET appveyor_token = $1
                                       WHERE id = $2"));
        try!(stmt.query(&[&token, &self.id]));
        Ok(())
    }

    pub fn from_row(row: &Row) -> Project {
        Project {
            id: row.get("id"),
            repo_user: row.get("repo_user"),
            repo_name: row.get("repo_name"),
            github_repo_id: row.get("github_repo_id"),
            github_access_token: row.get("github_access_token"),
            github_webhook_secret: row.get("github_webhook_secret"),
            appveyor_token: row.get("appveyor_token"),
            travis_access_token: row.get("travis_access_token"),
        }
    }
}
