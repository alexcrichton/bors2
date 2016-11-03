#[derive(RustcDecodable)]
pub struct GetRepository {
    pub repo: Repository,
}

#[derive(RustcDecodable)]
pub struct Repository {
    pub id: i32,
}
