use std::error::Error;
use std::sync::Arc;

use conduit::{Request, Response};
use conduit_middleware::Middleware;
use oauth2;
use r2d2;

use {db, Config};

/// The `App` struct holds the main components of the application like
/// the database connection pool and configurations
pub struct App {
    pub database: db::Pool,
    pub github: oauth2::Config,
    pub session_key: String,
    pub config: Config,
}

/// The `AppMiddleware` injects an `App` instance into the `Request` extensions
pub struct AppMiddleware {
    app: Arc<App>
}

impl App {
    pub fn new(config: &Config) -> App {
        let mut github = oauth2::Config::new(
            &config.gh_client_id,
            &config.gh_client_secret,
            "https://github.com/login/oauth/authorize",
            "https://github.com/login/oauth/access_token",
        );

        // generally useful
        github.scopes.push("user:email".to_string());
        // we're writing status descriptions
        github.scopes.push("repo:status".to_string());
        // we're updating repo hooks
        github.scopes.push("write:repo_hook".to_string());
        // we're updating code
        github.scopes.push("public_repo".to_string());
        // going to create an authorization
        github.scopes.push("user".to_string());
        github.scopes.push("admin:org".to_string());

        let db_config = r2d2::Config::builder()
            .pool_size(if config.env == ::Env::Production {10} else {1})
            .helper_threads(if config.env == ::Env::Production {3} else {1})
            .build();

        return App {
            database: db::pool(&config.db_url, db_config),
            github: github,
            session_key: config.session_key.clone(),
            config: config.clone(),
        };
    }
}

impl AppMiddleware {
    pub fn new(app: Arc<App>) -> AppMiddleware {
        AppMiddleware { app: app }
    }
}

impl Middleware for AppMiddleware {
    fn before(&self, req: &mut Request) -> Result<(), Box<Error+Send>> {
        req.mut_extensions().insert(self.app.clone());
        Ok(())
    }

    fn after(&self, req: &mut Request, res: Result<Response, Box<Error+Send>>)
             -> Result<Response, Box<Error+Send>> {
        req.mut_extensions().pop::<Arc<App>>().unwrap();
        res
    }
}

/// Adds an `app()` method to the `Request` type returning the global `App` instance
pub trait RequestApp {
    fn app(&self) -> &Arc<App>;
}

impl<'a> RequestApp for Request + 'a {
    fn app(&self) -> &Arc<App> {
        self.extensions().find::<Arc<App>>()
            .expect("Missing app")
    }
}

