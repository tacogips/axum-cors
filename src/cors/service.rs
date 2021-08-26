use super::config::*;

use axum::body::{box_body, BoxBody};
use bytes::Bytes;
use futures_util::ready;
use http::{self, HeaderMap, Request, Response, StatusCode};
use log::debug;
use pin_project_lite::pin_project;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tower::{BoxError, Service};

#[derive(Debug, Clone)]
pub struct CorsService<S> {
    inner: S,
    config: Arc<Config>,
}

impl<S> CorsService<S> {
    pub fn new(inner: S, config: Arc<Config>) -> CorsService<S> {
        CorsService { inner, config }
    }
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for CorsService<S>
where
    ReqBody: Send + 'static,
    ResBody: http_body::Body<Data = Bytes> + Send + Sync + 'static,
    ResBody::Error: Into<BoxError>,
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone,
{
    type Response = Response<BoxBody>;
    type Error = S::Error;
    type Future = CorsFuture<ReqBody, S>;

    #[inline]
    fn poll_ready(&mut self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(ctx)
    }

    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        let inner = match self.config.process_request(&request) {
            Ok(CorsResource::Preflight(headers)) => CorsFutureInner::Handled {
                headers: Some(headers),
            },
            Ok(CorsResource::Simple(headers)) => CorsFutureInner::Simple {
                future: self.inner.call(request),
                headers: Some(headers),
            },
            Err(e) => {
                debug!("CORS request to {} is denied: {:?}", request.uri(), e);
                CorsFutureInner::Handled { headers: None }
            }
        };

        CorsFuture { inner }
    }
}

pin_project! {
    pub struct CorsFuture<ReqBody, S>
    where
        S: Service<Request<ReqBody>>
    {
        #[pin] inner:CorsFutureInner<ReqBody,  S>
    }
}

impl<ReqBody, ResBody, S> Future for CorsFuture<ReqBody, S>
where
    ResBody: http_body::Body<Data = Bytes> + Send + Sync + 'static,
    ResBody::Error: Into<BoxError>,
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
{
    type Output = Result<Response<BoxBody>, S::Error>;
    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();
        this.inner.poll(ctx)
    }
}

pin_project! {

    #[allow(missing_debug_implementations)]
    #[project = CorsFutureInnerProj]
    enum CorsFutureInner<ReqBody, S>
    where
        S: Service<Request<ReqBody>>
    {
        Simple{#[pin]future:S::Future, headers:Option<HeaderMap>},
        Handled{headers:Option<HeaderMap>},
    }
}

impl<ReqBody, ResBody, S> Future for CorsFutureInner<ReqBody, S>
where
    ResBody: http_body::Body<Data = Bytes> + Send + Sync + 'static,
    ResBody::Error: Into<BoxError>,
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
{
    type Output = Result<Response<BoxBody>, S::Error>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();

        match this {
            CorsFutureInnerProj::Simple { future, headers } => {
                let response = ready!(future.poll(ctx));
                match response {
                    Ok(mut response) => {
                        let headers = headers.take().expect("poll called twice");
                        response.headers_mut().extend(headers);
                        Poll::Ready(Ok(response.map(box_body)))
                    }
                    Err(err) => Poll::Ready(Err(err)),
                }
            }
            CorsFutureInnerProj::Handled { headers } => {
                let mut response = http::Response::new(http_body::Full::new(Bytes::new()));
                *response.status_mut() = StatusCode::FORBIDDEN;

                if let Some(headers) = headers.take() {
                    *response.status_mut() = StatusCode::NO_CONTENT;
                    *response.headers_mut() = headers;
                }

                Poll::Ready(Ok(response.map(box_body)))
            }
        }
    }
}
