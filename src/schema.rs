table! {
    projects (id) {
        id -> i32,
        repo_name -> String,
        repo_user -> String,
        github_webhook_secret -> String,
        github_access_token -> String,
        travis_access_token -> Option<String>,
        appveyor_token -> Option<String>,
    }
}
