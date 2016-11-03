#[macro_use]
extern crate log;
#[macro_use]
extern crate error_chain;
extern crate conduit;
extern crate conduit_conditional_get;
extern crate conduit_cookie;
extern crate conduit_log_requests;
extern crate handlebars;
extern crate conduit_middleware;
extern crate conduit_router;
extern crate curl;
extern crate lazycell;
extern crate oauth2;
extern crate conduit_static;
extern crate openssl;
extern crate postgres as pg;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate rand;
extern crate base64;
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
use openssl::crypto::pkey::PKey;
use rustc_serialize::hex::ToHex;

use app::{App, RequestApp};
use db::RequestTransaction;
use errors::*;
use models::*;
use util::{RequestFlash, C};

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
pub mod appveyor;
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
    router.post("/repos/:user/:repo/add-travis-token", C(repo_add_travis));
    router.post("/repos/:user/:repo/add-appveyor-token", C(repo_add_appveyor));
    router.get("/authorize/github", C(authorize_github));
    router.post("/webhook/github/:user/:repo", C(github_webhook));
    router.post("/webhook/appveyor/:user/:repo", C(appveyor_webhook));
    router.post("/webhook/travis", C(travis_webhook));
    router.get("/assets/*path", conduit_static::Static::new("."));

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

fn repo_add_travis(req: &mut Request) -> BorsResult<Response> {
    let mut query = Vec::new();
    try!(req.body().read_to_end(&mut query));
    let query = url::form_urlencoded::parse(&query).collect::<Vec<_>>();

    let token = query.iter().find(|q| q.0 == "token").unwrap();
    let token = &token.1;

    let project = try!(req_project(req));

    let url = format!("/repos/{}/{}", project.repo_user, project.repo_name);
    let travis_repo: travis::GetRepository = match http::travis_get(&url, &token) {
        Ok(repo) => repo,
        Err(_) => {
            req.set_flash_error("travis token was invalid");
            return repo_show(req);
        }
    };

    let url = format!("/repos/{}/settings", travis_repo.repo.id);
    if http::travis_get::<travis::GetRepoSettings>(&url, &token).is_err() {
        req.set_flash_error("project not registered?");
        return repo_show(req);
    }

    try!(project.set_travis_token(try!(req.tx()), &token));

    Ok(util::redirect(&format!("/repos/{}/{}",
                               project.repo_user,
                               project.repo_name)))
}

fn repo_add_appveyor(req: &mut Request) -> BorsResult<Response> {
    let mut query = Vec::new();
    try!(req.body().read_to_end(&mut query));
    let query = url::form_urlencoded::parse(&query).collect::<Vec<_>>();

    let token = query.iter().find(|q| q.0 == "token").unwrap();
    let token = &token.1;
    let project = try!(req_project(req));

    // Test out the token by fetching the user's list of projects
    let url = format!("/projects");
    let projects: Vec<appveyor::Project> = match http::appveyor_get(&url, &token) {
        Ok(projects) => projects,
        Err(_) => {
            req.set_flash_error("appveyor token was invalid");
            return repo_show(req)
        }
    };
    let repo_name = format!("{}/{}", project.repo_user, project.repo_name);
    let appveyor_project = projects.iter().filter(|p| {
        p.repositoryType == "github" && p.repositoryName == repo_name
    }).next();

    // Register the project if it's not already registered
    if appveyor_project.is_none() {
        let new = appveyor::NewProject {
            repositoryProvider: "gitHub".to_string(),
            repositoryName: repo_name,
        };
        let project: appveyor::Project = try!(http::appveyor_post("/projects",
                                                                  &token,
                                                                  &new));
        drop(project);
    }

    // Ok, set the token and go back to the repo
    try!(project.set_appveyor_token(try!(req.tx()), &token));
    Ok(util::redirect(&format!("/repos/{}/{}",
                               project.repo_user,
                               project.repo_name)))
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

    Ok(site_html(req, &page))
}

fn repo_show(req: &mut Request) -> BorsResult<Response> {
    let project = try!(req_project(req));

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
            <form action='/repos/{repo_user}/{repo_name}/add-travis-token' \
                  method=post>
                <input type=text name=token placeholder='Enter travis token'/>
            </form>
        ",
        repo_user = project.repo_user,
        repo_name = project.repo_name));
    }

    if project.appveyor_token.is_none() {
        page.push_str(&format!("\
            <form action='/repos/{repo_user}/{repo_name}/add-appveyor-token' \
                  method=post>
                <input type=text name=token placeholder='Enter appveyor token'/>
            </form>
        ",
        repo_user = project.repo_user,
        repo_name = project.repo_name));
    }

    Ok(site_html(req, &page))
}

fn req_project(req: &Request) -> BorsResult<Project> {
    let user = &req.params()["user"];
    let repo = &req.params()["repo"];
    Project::find_by_name(try!(req.tx()), user, repo)
}

fn github_webhook(req: &mut Request) -> BorsResult<Response> {
    let event = req.headers().find("X-GitHub-Event")
                   .expect("event not present")[0].to_string();
    let signature = req.headers().find("X-Hub-Signature")
                       .expect("signature not present")[0].to_string();
    let id = req.headers().find("X-GitHub-Delivery")
                .expect("delivery not present")[0].to_string();

    let mut body = Vec::new();
    try!(req.body().read_to_end(&mut body));

    let tx = try!(req.tx());
    let project = try!(req_project(req));

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

fn travis_webhook(req: &mut Request) -> BorsResult<Response> {
    let slug = req.headers().find("Travis-Repo-Slug")
                  .expect("slug not present")[0].to_string();
    let signature = req.headers().find("Signature")
                       .expect("signature not present")[0].to_string();
    let mut body = Vec::new();
    try!(req.body().read_to_end(&mut body));
    let query = url::form_urlencoded::parse(&body).collect::<Vec<_>>();
    let payload = &query.iter().find(|q| q.0 == "payload").unwrap().1;
    let signature = try!(base64::decode(&signature).chain_err(|| {
        "signature was not valid base64"
    }));

    let url = "https://api.travis-ci.org/config";
    let config: travis::GetConfig = try!(http::get(url, &[]).chain_err(|| {
        "failed to get travis config"
    }));
    let key = config.config.notifications.webhook.public_key;
    let key = try!(PKey::public_key_from_pem(&key.as_bytes()).chain_err(|| {
        "key was not valid pem"
    }));
    let rsa = try!(key.get_rsa().chain_err(|| "not an rsa key"));
    println!("{:?}", signature);
    println!("{:?}", payload);
    try!(rsa.verify(Type::SHA1, payload.as_bytes(), &signature).chain_err(|| {
        "invalid signature"
    }));

    let mut parts = slug.splitn(2, '/');
    let repo_user = parts.next().unwrap();
    let repo_name = parts.next().unwrap();

    // Verify this is one of our projects
    let project = try!(Project::find_by_name(try!(req.tx()),
                                             repo_user,
                                             repo_name));
    drop(project);

    try!(Event::insert(try!(req.tx()), Provider::Travis, "", "", payload));

    Ok(util::html(""))
}

fn appveyor_webhook(req: &mut Request) -> BorsResult<Response> {
    panic!()
}

fn site_html(req: &Request, body: &str) -> Response {
    let mut page = format!(r#"
<html>
<head>
<link href="/assets/site.css" rel=stylesheet>
</head>
<body>
    "#);

    if let Some(error) = req.flash_error() {
        page.push_str(&format!("\
            <div class='flash error'>{}</div>
        ", handlebars::html_escape(error)));
    }

    page.push_str(body);
    page.push_str("
</body>
</html>
");

    util::html(&page)
}

pub struct R404(pub RouteBuilder);

impl Handler for R404 {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Error+Send>> {
        let R404(ref router) = *self;
        let res = match router.recognize(&req.method(), req.path()) {
            Ok(m) => {
                req.mut_extensions().insert(m.params.clone());
                m.handler.call(req)
            }
            Err(_) => {
                let mut response = site_html(req, "page not found");
                response.status = (404, "Not Found");
                Ok(response)
            }
        };

        let err = match res {
            Ok(e) => return Ok(e),
            Err(e) => {
                match e.downcast::<BorsError>() {
                    Ok(err) => err,
                    Err(e) => return Err(e),
                }
            }
        };

        match *err.kind() {
            BorsErrorKind::MissingProject => {
                req.set_flash_error("user/repo combo not found");
                repos(req).map_err(|e| Box::new(e) as Box<_>)
            }
            _ => {
                {
                    error!("top-level error: {}", err);
                    let mut cur = err.cause();
                    while let Some(e) = cur {
                        error!("error: {}", e);
                        cur = e.cause();
                    }
                }
                Err(Box::new(err))
            }
        }
    }
}
