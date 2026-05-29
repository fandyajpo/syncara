pub mod pattern;

use crate::config::{Config, RouteConfig};

/// Pre-compiled routing table built from configuration.
#[derive(Clone)]
pub struct Router {
    routes: Vec<RouteConfig>,
}

/// The result of a route match.
pub struct RouteMatch {
    /// The matching route definition.
    pub route: RouteConfig,

    /// The pool name to forward to.
    pub pool: String,
}

impl Router {
    pub fn new(config: &Config) -> Self {
        Self {
            routes: config.routes.clone(),
        }
    }

    /// Match an incoming request to a route.
    ///
    /// Iterates routes in **config file order** and returns the first match.
    /// Returns `None` if no route matches the request.
    pub fn route(&self, host: Option<&str>, path: &str) -> Option<RouteMatch> {
        for route in &self.routes {
            let host_match = match &route.host {
                Some(pattern) => match host {
                    Some(h) => pattern::match_host(pattern, h),
                    None => false,
                },
                None => true,
            };

            let path_match = match &route.path {
                Some(pattern) => pattern::match_path_prefix(pattern, path),
                None => true,
            };

            if host_match && path_match {
                return Some(RouteMatch {
                    route: route.clone(),
                    pool: route.pool.clone(),
                });
            }
        }

        None
    }

    /// Rebuild route table from a new config (called on hot reload).
    pub fn rebuild(&mut self, config: &Config) {
        self.routes = config.routes.clone();
    }
}
