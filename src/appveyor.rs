#![allow(bad_style)]

#[derive(RustcDecodable)]
pub struct Project {
    pub projectId: u32,
    pub repositoryType: String,
    pub slug: String,
    pub name: String,
    pub repositoryName: String,
}

#[derive(RustcEncodable)]
pub struct NewProject {
    pub repositoryProvider: String,
    pub repositoryName: String,
}
