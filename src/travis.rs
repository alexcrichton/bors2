#[derive(RustcDecodable)]
pub struct GetRepository {
    pub repo: Repository,
}

#[derive(RustcDecodable)]
pub struct Repository {
    pub id: i32,
}

#[derive(RustcDecodable)]
pub struct GetRepoSettings {
    pub settings: RepoSettings,
}

#[derive(RustcDecodable)]
pub struct RepoSettings {
    pub maximum_number_of_builds: u32,
}
