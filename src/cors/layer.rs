use super::config::Config;
use super::service::CorsService;
use std::sync::Arc;

use tower::Layer;

pub struct CorsLayer {
    config: Arc<Config>,
}

impl CorsLayer {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }
}

impl<S> Layer<S> for CorsLayer {
    type Service = CorsService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        CorsService::new(inner, self.config.clone())
    }
}
