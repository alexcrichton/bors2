#[macro_use]
extern crate log;
#[macro_use]
extern crate error_chain;
extern crate rustc_serialize;
extern crate curl;
extern crate oauth2;
extern crate r2d2;
extern crate postgres as pg;
extern crate r2d2_postgres;
extern crate conduit_middleware;
extern crate lazycell;
extern crate conduit_router;
extern crate conduit_conditional_get;
extern crate conduit_cookie;
extern crate conduit_log_requests;
extern crate conduit;

use std::sync::Arc;
use std::error::Error;

use conduit_middleware::MiddlewareBuilder;
use conduit::{Request, Response};
use conduit_router::RouteBuilder;

use app::App;
use errors::*;

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

// pub mod models;
pub mod app;
pub mod db;
pub mod http;
pub mod github;
pub mod errors;
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

    router.post("/add-repo", add_repo);
    // router.get("/authorize/github", authorize_github);
    // router.get("/", index);

    let env = app.config.env;
    let mut m = MiddlewareBuilder::new(router);
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

fn add_repo(_req: &mut Request) -> BorsResult<Response> {
    loop {}
    // let repo = url.query_pairs()
    //               .find(|&(ref a, _)| a == "repo")
    //               .map(|(_, value)| value)
    //               .expect("repo not present in url");
    // let oauth = self.oauth();
    // let redirect_url = oauth.authorize_url(repo.to_string());
    // debug!("oauth redirect to {}", redirect_url);
    // res.headers_mut().set(Location(redirect_url.to_string()));
    // *res.status_mut() = StatusCode::Found;
    // Ok(Vec::new())
}

//     fn authorize_github(&self, req: Request, res: &mut Response, url: &Url)
//                         -> Result<Vec<u8>> {
//         let code = url.query_pairs()
//                       .find(|&(ref a, _)| a == "code")
//                       .map(|(_, value)| value)
//                       .expect("code not present in url");
//         let state = url.query_pairs()
//                        .find(|&(ref a, _)| a == "state")
//                        .map(|(_, value)| value)
//                        .expect("state not present in url");
//         match self.add_project(&code, &state) {
//             Ok(()) => {}
//             Err(e) => return Ok(self.fail(e)),
//         }
//         self.index(req, res)
//     }
//
//     fn add_project(&self, code: &str, repo_name: &str) -> Result<()> {
//         let github_access_token = try!(self.oauth().exchange(code.to_string()));
//
//         // let travis_token = try!(self.negotiate_travis_token(&github_access_token));
//         // println!("travis token: {}", travis_token);
//
//         let mut parts = repo_name.splitn(2, '/');
//         let user = parts.next().unwrap();
//         let name = parts.next().unwrap();
//         let github_webhook_secret = thread_rng().gen_ascii_chars().take(20)
//                                                 .collect::<String>();
//
//         try!(self.add_github_webhook_to_bors2(&github_access_token,
//                                               user,
//                                               name,
//                                               &github_webhook_secret));
//
//         // let new_project = NewProject {
//         //     repo_user: user,
//         //     repo_name: name,
//         //     github_access_token: &github_access_token.access_token,
//         //     github_webhook_secret: &github_webhook_secret,
//         // };
//         // let conn = bors2::establish_connection();
//         // let project: Project = try!(diesel::insert(&new_project)
//         //                                    .into(projects::table)
//         //                                    .get_result(conn));
//         // drop(project);
//
//         Ok(())
//     }
//
//     // fn negotiate_travis_token(&self, github_access_token: &oauth2::Token)
//     //                           -> Result<String> {
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
//
//     fn add_github_webhook_to_bors2(&self,
//                                    token: &oauth2::Token,
//                                    user: &str,
//                                    repo: &str,
//                                    secret: &str) -> Result<()> {
//         let url = format!("/repos/{}/{}/hooks", user, repo);
//         let w: Webhook = try!(github_post(&url, &token, &CreateWebhook {
//             name: "web".to_string(),
//             active: true,
//             events: vec![
//             ],
//             config: CreateWebhookConfig {
//                 content_type: "json".to_string(),
//                 url: format!("{}/github-webhook", self.host),
//                 secret: secret.to_string(),
//             },
//         }));
//         drop(w);
//         Ok(())
//     }

fn index(_req: &mut Request) -> BorsResult<Response> {
//     let body = format!(r#"
// <html>
// <body>
// <form action="/add-repo">
// Add repo: <input name=repo type=text />
// </form>
// </body>
// </html>
// "#);
//     Ok(body.into())
    loop {}
}

//     fn not_found(&self) -> Vec<u8> {
//         br#"
//             <html>
//             <body>
//             Not found
//             </body>
//             </html>
//         "#.to_vec()
//     }
//
//     fn fail(&self, e: Error) -> Vec<u8> {
//         format!(r#"
//             <html>
//             <body>
//             error in last request: {}
//             </body>
//             </html>
//         "#, handlebars::html_escape(&format!("{:?}", e))).into()
//     }
// }
