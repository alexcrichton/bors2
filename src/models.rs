use schema::*;

#[derive(Queryable)]
pub struct PullRequest {
    pub id: i32,
}

#[derive(Queryable)]
pub struct Project {
    pub id: i32,
    pub repo_user: String,
    pub repo_name: String,
    pub github_access_token: String,
    pub github_webhook_secret: String,
    pub appveyor_token: Option<String>,
    pub travis_access_token: Option<String>,
}

#[derive(Insertable)]
#[table_name="projects"]
pub struct NewProject<'a> {
    pub repo_user: &'a str,
    pub repo_name: &'a str,
    pub github_access_token: &'a str,
    pub github_webhook_secret: &'a str,
}
