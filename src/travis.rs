#[derive(RustcEncodable)]
pub struct TravisAuthGithub {
    pub github_token: String,
}

#[derive(RustcDecodable)]
pub struct TravisAccessToken {
    pub access_token: String,
}
