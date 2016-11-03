use std::collections::HashMap;

#[derive(RustcDecodable, Debug)]
pub struct Webhook {
    pub id: i32,
    pub url: String,
    pub name: String,
    pub events: Vec<String>,
    pub active: bool,
    pub config: HashMap<String, String>,
    pub updated_at: String,
    pub created_at: String,
}

#[derive(RustcEncodable)]
pub struct CreateWebhook {
    pub name: String,
    pub config: CreateWebhookConfig,
    pub events: Vec<String>,
    pub active: bool,
}

#[derive(RustcEncodable)]
pub struct CreateWebhookConfig {
    pub url: String,
    pub content_type: String,
    pub secret: String,
}

#[derive(RustcEncodable)]
pub struct CreateAuthorization {
    pub scopes: Vec<String>,
    pub note: String,
}

#[derive(RustcDecodable)]
pub struct Authorization {
    pub id: i32,
    pub url: String,
    pub scopes: Vec<String>,
    pub token: String,
}

#[derive(RustcDecodable)]
pub struct Repository {
    pub id: i32,
    pub name: String,
}
