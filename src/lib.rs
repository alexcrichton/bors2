#[macro_use]
extern crate log;
#[macro_use]
extern crate error_chain;
extern crate conduit;
extern crate conduit_conditional_get;
extern crate conduit_cookie;
extern crate conduit_log_requests;
extern crate conduit_middleware;
extern crate conduit_router;
extern crate curl;
extern crate lazycell;
extern crate oauth2;
extern crate openssl;
extern crate postgres as pg;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate rand;
extern crate rustc_serialize;
extern crate url;

use std::error::Error;
use std::str;
use std::sync::Arc;

use conduit::{Request, Response, Handler};
use conduit_middleware::MiddlewareBuilder;
use conduit_router::{RouteBuilder, RequestParams};
use rand::{Rng, thread_rng};
use openssl::crypto::hmac;
use openssl::crypto::hash::Type;
use rustc_serialize::hex::ToHex;

use app::{App, RequestApp};
use db::RequestTransaction;
use errors::*;
use models::*;
use util::C;

#[derive(Clone)]
pub struct Config {
    pub session_key: String,
    pub gh_client_id: String,
    pub gh_client_secret: String,
    pub db_url: String,
    pub env: ::Env,
    pub host: String,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Env {
    Development,
    Test,
    Production,
}

pub mod app;
pub mod db;
pub mod errors;
pub mod github;
pub mod http;
pub mod models;
pub mod travis;
pub mod util;

pub fn env(s: &str) -> String {
    match std::env::var(s) {
        Ok(s) => s,
        Err(_) => panic!("must have `{}` defined", s),
    }
}

pub fn middleware(app: Arc<App>) -> MiddlewareBuilder {
    let mut router = RouteBuilder::new();

    router.get("/", C(repos));
    router.post("/repos", C(repo_new));
    router.get("/repos/:user/:repo", C(repo_show));
    router.post("/repos/:user/:repo/travis-auth", C(authorize_travis));
    router.get("/authorize/github", C(authorize_github));
    router.post("/webhook/github/:user/:repo", C(github_webhook));

    let env = app.config.env;
    let mut m = MiddlewareBuilder::new(R404(router));
    if env == Env::Development {
        m.add(DebugMiddleware);
    }
    if env != Env::Test {
        m.add(conduit_log_requests::LogRequests(log::LogLevel::Info));
    }
    m.add(conduit_conditional_get::ConditionalGet);
    m.add(conduit_cookie::Middleware::new(app.session_key.as_bytes()));
    m.add(conduit_cookie::SessionMiddleware::new("bors2_session",
                                                 env == Env::Production));
    m.add(app::AppMiddleware::new(app));
    if env != Env::Test {
        m.add(db::TransactionMiddleware);
    }

    return m;

    struct DebugMiddleware;

    impl conduit_middleware::Middleware for DebugMiddleware {
        fn before(&self, req: &mut conduit::Request)
                  -> Result<(), Box<Error+Send>> {
            println!("  version: {}", req.http_version());
            println!("  method: {:?}", req.method());
            println!("  scheme: {:?}", req.scheme());
            println!("  host: {:?}", req.host());
            println!("  path: {}", req.path());
            println!("  query_string: {:?}", req.query_string());
            println!("  remote_addr: {:?}", req.remote_addr());
            for &(k, ref v) in req.headers().all().iter() {
                println!("  hdr: {}={:?}", k, v);
            }
            Ok(())
        }
        fn after(&self, _req: &mut conduit::Request,
                 res: Result<conduit::Response, Box<Error+Send>>)
                 -> Result<conduit::Response, Box<Error+Send>> {
            res.map(|res| {
                println!("  <- {:?}", res.status);
                for (k, v) in res.headers.iter() {
                    println!("  <- {} {:?}", k, v);
                }
                res
            })
        }
    }
}

fn repo_new(req: &mut Request) -> BorsResult<Response> {
    let mut query = Vec::new();
    try!(req.body().read_to_end(&mut query));

    let mut query = url::form_urlencoded::parse(&query);
    let repo = query.find(|&(ref a, _)| a == "repo")
                    .map(|(_, value)| value)
                    .expect("failed to find `repo` in query string");
    let app = req.app();
    let redirect_url = app.github.authorize_url(repo.into_owned());
    debug!("oauth redirect to {}", redirect_url);
    Ok(util::redirect(&redirect_url.to_string()))
}

fn authorize_github(req: &mut Request) -> BorsResult<Response> {
    let query = req.query_string().unwrap_or("").to_string();
    let query = url::form_urlencoded::parse(query.as_bytes()).collect::<Vec<_>>();
    let code = query.iter()
                    .find(|&&(ref a, _)| a == "code")
                    .map(|&(_, ref value)| &value[..])
                    .expect("code not present in url");
    let state = query.iter()
                     .find(|&&(ref a, _)| a == "state")
                     .map(|&(_, ref value)| &value[..])
                     .expect("state not present in url");
    try!(add_project(req, &code, &state).chain_err(|| {
        "failed to add project"
    }));
    Ok(util::redirect("/"))
}

fn add_project(req: &mut Request, code: &str, repo_name: &str) -> BorsResult<()> {
    let github_access_token = try!(req.app().github.exchange(code.to_string()));

    let url = format!("/repos/{}", repo_name);
    let repo: github::Repository = try!(http::github_get(&url,
                                                         &github_access_token));

    let mut parts = repo_name.splitn(2, '/');
    let user = parts.next().unwrap();
    let name = parts.next().unwrap();
    let github_webhook_secret = thread_rng().gen_ascii_chars().take(20)
                                            .collect::<String>();

    try!(add_github_webhook_to_bors2(req.app(),
                                     &github_access_token,
                                     user,
                                     name,
                                     &github_webhook_secret));

    try!(Project::insert(try!(req.tx()),
                         user,
                         name,
                         repo.id,
                         &github_access_token.access_token,
                         &github_webhook_secret));
    Ok(())
}
//
//     // fn negotiate_travis_token(&self, github_access_token: &oauth2::Token)
//     //                           -> BorsResult<String> {
//     //     // let auth: Authorization = try!(github_post("/authorizations",
//     //     //                                            &github_access_token,
//     //     //                                            &CreateAuthorization {
//     //     //     // see https://docs.travis-ci.com/api#creating-a-temporary-github-token
//     //     //     scopes: vec![
//     //     //         "read:org".into(),
//     //     //         "user:email".into(),
//     //     //         "repo_deployment".into(),
//     //     //         "repo:status".into(),
//     //     //         "write:repo_hook".into(),
//     //     //     ],
//     //     //     note: "temporary token to auth against travis".to_string(),
//     //     // }));
//     //
//     //     let travis_headers = vec![
//     //         format!("Accept: application/vnd.travis-ci.2+json"),
//     //         format!("Content-Type: application/json"),
//     //     ];
//     //     let url = "https://api.travis-ci.org/auth/github";
//     //     let travis_auth: TravisAccessToken = try!(post(url,
//     //                                                    &travis_headers,
//     //                                                    &TravisAuthGithub {
//     //         github_token: github_access_token.access_token.clone(),
//     //     }));
//     //
//     //     // try!(github_delete(&auth.url, &github_access_token));
//     //
//     //     Ok(travis_auth.access_token)
//     // }

fn add_github_webhook_to_bors2(app: &App,
                               token: &oauth2::Token,
                               user: &str,
                               repo: &str,
                               secret: &str) -> BorsResult<()> {
    let url = format!("/repos/{}/{}/hooks", user, repo);
    let webhook = github::CreateWebhook {
        name: "web".to_string(),
        active: true,
        events: vec![
            "issue_comment".to_string(),
            "issues".to_string(),
            "pull_request".to_string(),
            "pull_request_review".to_string(),
            "pull_request_review_comment".to_string(),
            "status".to_string(),
        ],
        config: github::CreateWebhookConfig {
            content_type: "json".to_string(),
            url: format!("{}/webhook/github/{}/{}", app.config.host, user, repo),
            secret: secret.to_string(),
        },
    };
    let w: github::Webhook = try!(http::github_post(&url, &token, &webhook));
    drop(w);
    Ok(())
}

fn authorize_travis(req: &mut Request) -> BorsResult<Response> {
    let mut query = Vec::new();
    try!(req.body().read_to_end(&mut query));
    let query = url::form_urlencoded::parse(&query).collect::<Vec<_>>();
    let token = query.iter().find(|pair| {
        pair.0 == "token"
    }).expect("token not found in body");
    let token = &token.1;
    let user = req.params()["user"].to_string();
    let repo = req.params()["repo"].to_string();

    let url = format!("/repos/{}/{}", user, repo);
    println!("{:?}", query);
    println!("{} {}", url, token);
    let repo: travis::GetRepository = try!(http::travis_get(&url, &token));

    Ok(util::redirect("/"))
}

fn repos(req: &mut Request) -> BorsResult<Response> {
    let tx = try!(req.tx());
    let projects = try!(Project::all(tx));
    let mut page = format!(r#"
<form action="/repos" method=post>
Add repo: <input name=repo type=text />
</form>

<table>
"#);

    for project in projects {
        page.push_str(&format!("<tr>\
            <td>\
                <a href='/repos/{user}/{name}'>{user}/{name}</a>
            </td>\
        </tr>\n",
        user = project.repo_user,
        name = project.repo_name,
        ));
    }

    page.push_str("\n</table>");

    Ok(site_html(&page))
}

fn repo_show(req: &mut Request) -> BorsResult<Response> {
    let user = req.params()["user"].to_string();
    let repo = req.params()["repo"].to_string();
    let app = req.app();
    let tx = try!(req.tx());
    let project = match try!(Project::find_by_name(tx, &user, &repo)) {
        Some(project) => project,
        None => return Err("no project found".into()),
    };

    let mut page = format!("\
<h2>\
    <a href='https://github.com/{repo_user}/{repo_name}'>\
        {repo_user}/{repo_name}\
    </a>\
</h2>
",
        repo_name = project.repo_name,
        repo_user = project.repo_user);

    if project.travis_access_token.is_none() {
        page.push_str(&format!("\
            <a href='https://api.travis-ci.com/auth/handshake?redirect_uri={redirect}'>\
                Authenticate Travis CI\
            </a>
        ",
        redirect = format!("{}/repos/{}/{}/travis-auth",
                           app.config.host,
                           project.repo_user,
                           project.repo_name)));
    }

    Ok(site_html(&page))
}

fn github_webhook(req: &mut Request) -> BorsResult<Response> {
    let event = req.headers().find("X-GitHub-Event")
                   .expect("event not present")[0].to_string();
    let signature = req.headers().find("X-Hub-Signature")
                       .expect("signature not present")[0].to_string();
    let id = req.headers().find("X-GitHub-Delivery")
                .expect("delivery not present")[0].to_string();
    let user = req.params()["user"].to_string();
    let repo = req.params()["repo"].to_string();

    let mut body = Vec::new();
    try!(req.body().read_to_end(&mut body));

    let tx = try!(req.tx());
    let project = match try!(Project::find_by_name(tx, &user, &repo)) {
        Some(project) => project,
        None => return Err("no project found".into()),
    };

    let my_signature = try!(hmac::hmac(Type::SHA1,
                                       project.github_webhook_secret.as_bytes(),
                                       &body));
    let my_signature = format!("sha1={}", my_signature.to_hex());
    if !openssl::crypto::memcmp::eq(signature.as_bytes(), my_signature.as_bytes()) {
        return Err("invalid signature".into())
    }

    try!(Event::insert(tx, Provider::GitHub, &id, &event,
                       try!(str::from_utf8(&body))));
    Ok(util::html(""))
}

fn site_html(body: &str) -> Response {
    util::html(&format!(r#"
<html>
<body>
{}
</body>
</html>
"#, body))
}

pub struct R404(pub RouteBuilder);

impl Handler for R404 {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Error+Send>> {
        let R404(ref router) = *self;
        match router.recognize(&req.method(), req.path()) {
            Ok(m) => {
                req.mut_extensions().insert(m.params.clone());
                m.handler.call(req)
            }
            Err(..) => {
                let mut response = site_html("page not found");
                response.status = (404, "Not Found");
                return Ok(response)
            }
        }
    }
}
