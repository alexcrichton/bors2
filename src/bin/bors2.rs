extern crate bors2;
extern crate env_logger;
extern crate diesel;
extern crate handlebars;
extern crate hyper;
extern crate oauth2;
extern crate rand;
extern crate rustc_serialize;
extern crate url;

#[macro_use]
extern crate log;

use std::panic;
use std::env;
use std::net::SocketAddr;
use std::fs::File;

use bors2::errors::*;
use bors2::github::*;
use bors2::http::*;
use bors2::models::*;
use bors2::schema::*;
use bors2::travis::*;
use diesel::prelude::*;
use hyper::header::Location;
use hyper::server::{Server, Handler, Request, Response};
use hyper::status::StatusCode;
use hyper::uri::RequestUri;
use rand::{thread_rng, Rng};
use url::Url;

fn main() {
    env_logger::init().unwrap();
    let gh_client_id = env::var("GH_CLIENT_ID").expect("GH_CLIENT_ID env var");
    let gh_client_secret = env::var("GH_CLIENT_SECRET").expect("GH_CLIENT_SECRET env var");

    let heroku = env::var("HEROKU").is_ok();
    if heroku {
        File::create("/tmp/app-initialized").unwrap();
    }
    let host = if heroku {
        format!("https://bors2-test.herokuapp.com")
    } else {
        format!("http://localhost:3000")
    };

    let addr = env::args().nth(1).unwrap_or("127.0.0.1:3000".to_string());
    let addr = addr.parse::<SocketAddr>().unwrap();
    let app = App {
        gh_client_id: gh_client_id,
        gh_client_secret: gh_client_secret,
        heroku: heroku,
        host: host,
    };
    Server::http(addr).unwrap().handle(app).unwrap();
}

struct App {
    gh_client_id: String,
    gh_client_secret: String,
    heroku: bool,
    host: String,
}

impl App {
    fn oauth(&self) -> oauth2::Config {
        let mut config = oauth2::Config::new(&self.gh_client_id,
                                             &self.gh_client_secret,
                                             "https://github.com/login/oauth/authorize",
                                             "https://github.com/login/oauth/access_token");

        // generally useful
        config.scopes.push("user:email".to_string());
        // we're writing status descriptions
        config.scopes.push("repo:status".to_string());
        // we're updating repo hooks
        config.scopes.push("write:repo_hook".to_string());
        // we're updating code
        config.scopes.push("public_repo".to_string());
        // going to create an authorization
        config.scopes.push("user".to_string());
        config.scopes.push("admin:org".to_string());


        // travis claims to want these
        // config.scopes.push("read:org", "user:email", "repo_deployment",
        //         "repo:status", "write:repo_hook"
        // config.redirect_url = format!("{}:{}/github-callback",
        //                               if self.heroku {"https"} else {"http"},
        //                               req.headers.get::<Host>().unwrap());
        return config
    }

    fn dispatch(&self, req: Request, res: &mut Response) -> Result<Vec<u8>> {
        debug!("got a request!");
        debug!("remote addr: {}", req.remote_addr);
        debug!("methods: {}", req.method);
        debug!("headers: {}", req.headers);
        debug!("uri: {}", req.uri);
        debug!("version: {}", req.version);

        let uri = match req.uri {
            RequestUri::AbsolutePath(ref s) => s.clone(),
            _ => return Err("only http path requests allowed".into()),
        };
        let url = format!("http://{}{}", req.remote_addr, uri);
        let url = Url::parse(&url).unwrap();
        match url.path() {
            "/add-repo" => self.add_repo(req, res, &url),
            "/authorize/github" => self.authorize_github(req, res, &url),
            "/" => self.index(req, res),
            _ => {
                *res.status_mut() = StatusCode::NotFound;
                return Ok(self.not_found())
            }
        }
    }

    fn add_repo(&self, _req: Request, res: &mut Response, url: &Url)
                -> Result<Vec<u8>> {
        let repo = url.query_pairs()
                      .find(|&(ref a, _)| a == "repo")
                      .map(|(_, value)| value)
                      .expect("repo not present in url");
        let oauth = self.oauth();
		let redirect_url = oauth.authorize_url(repo.to_string());
        debug!("oauth redirect to {}", redirect_url);
        res.headers_mut().set(Location(redirect_url.to_string()));
        *res.status_mut() = StatusCode::Found;
        Ok(Vec::new())
    }

    fn authorize_github(&self, req: Request, res: &mut Response, url: &Url)
                        -> Result<Vec<u8>> {
        let code = url.query_pairs()
                      .find(|&(ref a, _)| a == "code")
                      .map(|(_, value)| value)
                      .expect("code not present in url");
        let state = url.query_pairs()
                       .find(|&(ref a, _)| a == "state")
                       .map(|(_, value)| value)
                       .expect("state not present in url");
        match self.add_project(&code, &state) {
            Ok(()) => {}
            Err(e) => return Ok(self.fail(e)),
        }
        self.index(req, res)
    }

    fn add_project(&self, code: &str, repo_name: &str) -> Result<()> {
        let github_access_token = try!(self.oauth().exchange(code.to_string()));

        // let travis_token = try!(self.negotiate_travis_token(&github_access_token));
        // println!("travis token: {}", travis_token);

        let mut parts = repo_name.splitn(2, '/');
        let user = parts.next().unwrap();
        let name = parts.next().unwrap();
        let github_webhook_secret = thread_rng().gen_ascii_chars().take(20)
                                                .collect::<String>();

        try!(self.add_github_webhook_to_bors2(&github_access_token,
                                              user,
                                              name,
                                              &github_webhook_secret));

        let new_project = NewProject {
            repo_user: user,
            repo_name: name,
            github_access_token: &github_access_token.access_token,
            github_webhook_secret: &github_webhook_secret,
        };
        let conn = bors2::establish_connection();
        let project: Project = try!(diesel::insert(&new_project)
                                           .into(projects::table)
                                           .get_result(conn));
        drop(project);

        Ok(())
    }

    // fn negotiate_travis_token(&self, github_access_token: &oauth2::Token)
    //                           -> Result<String> {
    //     // let auth: Authorization = try!(github_post("/authorizations",
    //     //                                            &github_access_token,
    //     //                                            &CreateAuthorization {
    //     //     // see https://docs.travis-ci.com/api#creating-a-temporary-github-token
    //     //     scopes: vec![
    //     //         "read:org".into(),
    //     //         "user:email".into(),
    //     //         "repo_deployment".into(),
    //     //         "repo:status".into(),
    //     //         "write:repo_hook".into(),
    //     //     ],
    //     //     note: "temporary token to auth against travis".to_string(),
    //     // }));
    //
    //     let travis_headers = vec![
    //         format!("Accept: application/vnd.travis-ci.2+json"),
    //         format!("Content-Type: application/json"),
    //     ];
    //     let url = "https://api.travis-ci.org/auth/github";
    //     let travis_auth: TravisAccessToken = try!(post(url,
    //                                                    &travis_headers,
    //                                                    &TravisAuthGithub {
    //         github_token: github_access_token.access_token.clone(),
    //     }));
    //
    //     // try!(github_delete(&auth.url, &github_access_token));
    //
    //     Ok(travis_auth.access_token)
    // }

    fn add_github_webhook_to_bors2(&self,
                                   token: &oauth2::Token,
                                   user: &str,
                                   repo: &str,
                                   secret: &str) -> Result<()> {
        let url = format!("/repos/{}/{}/hooks", user, repo);
        let w: Webhook = try!(github_post(&url, &token, &CreateWebhook {
            name: "web".to_string(),
            active: true,
            events: vec![
            ],
            config: CreateWebhookConfig {
                content_type: "json".to_string(),
                url: format!("{}/github-webhook", self.host),
                secret: secret.to_string(),
            },
        }));
        drop(w);
        Ok(())
    }

    fn index(&self, _req: Request, _res: &mut Response) -> Result<Vec<u8>> {
        let body = format!(r#"
    <html>
    <body>
    <form action="/add-repo">
    Add repo: <input name=repo type=text />
    </form>
    </body>
    </html>
    "#);
        Ok(body.into())
    }

    fn not_found(&self) -> Vec<u8> {
        br#"
            <html>
            <body>
            Not found
            </body>
            </html>
        "#.to_vec()
    }

    fn fail(&self, e: Error) -> Vec<u8> {
        format!(r#"
            <html>
            <body>
            error in last request: {}
            </body>
            </html>
        "#, handlebars::html_escape(&format!("{:?}", e))).into()
    }
}

impl Handler for App {
    fn handle<'a, 'k>(&'a self, req: Request<'a, 'k>, mut res: Response<'a>) {
        drop(panic::catch_unwind(panic::AssertUnwindSafe(|| {
            match self.dispatch(req, &mut res) {
                Ok(body) => res.send(&body).unwrap(),
                Err(e) => {
                    error!("error during request: {:?}", e);
                    *res.status_mut() = StatusCode::InternalServerError;
                }
            }
        })));
    }
}
