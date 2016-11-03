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

#[derive(RustcDecodable)]
pub struct GetConfig {
    pub config: Config,
}

#[derive(RustcDecodable)]
pub struct Config {
    pub notifications: Notifications,
}

#[derive(RustcDecodable)]
pub struct Notifications {
    pub webhook: Webhook,
}

#[derive(RustcDecodable)]
pub struct Webhook {
    pub public_key: String,
}
