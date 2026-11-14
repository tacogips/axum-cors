pub mod cors;

pub use cors::*;

#[cfg(test)]
mod test {
    use http::{
        self,
        header::{self, HeaderName, HeaderValue},
        HeaderMap, Method,
    };
    use std::collections::BTreeSet;
    use std::iter::FromIterator;
    use std::time::Duration;

    pub use super::builder::*;
    pub use super::config::*;
    pub use super::layer::*;
    pub use super::service::*;

    type TestError = Box<dyn (::std::error::Error)>;
    type TestResult<T = ()> = ::std::result::Result<T, TestError>;

    macro_rules! assert_variant {
        ($value:expr, $var:pat) => {
            match $value {
                $var => {}
                _ => assert!(
                    false,
                    "Expected variant {}, was {:?}",
                    stringify!($var),
                    $value
                ),
            }
        };
    }

    macro_rules! assert_set {
        ($header:expr, $($val:expr),+) => {
            let actual = BTreeSet::from_iter($header.to_str()?.split(","));
            let expected = BTreeSet::from_iter(vec![$($val),+]);
            assert_eq!(actual, expected);
        }
    }

    impl CorsResource {
        fn into_simple(self) -> TestResult<HeaderMap> {
            match self {
                CorsResource::Simple(h) => Ok(h),
                _ => Err("Not a simple resource".into()),
            }
        }

        fn into_preflight(self) -> TestResult<HeaderMap> {
            match self {
                CorsResource::Preflight(h) => Ok(h),
                _ => Err("Not a preflight resource".into()),
            }
        }
    }

    #[test]
    fn simple_allows_when_origin_is_any() -> TestResult {
        common_allows_when_origin_is_any(
            simple_origin_config_builder(),
            simple_origin_request_builder,
        )
    }

    #[test]
    fn simple_disallows_null_origin_even_for_any() -> TestResult {
        common_disallows_null_origin_even_for_any(
            simple_origin_config_builder(),
            simple_origin_request_builder,
        )
    }

    #[test]
    fn simple_allows_null_origin_for_any_when_configured() -> TestResult {
        common_allows_null_origin_for_any_when_configured(
            simple_origin_config_builder(),
            simple_origin_request_builder,
        )
    }

    #[test]
    fn simple_compares_origin_against_allowed_origins() -> TestResult {
        common_compares_origin_against_allowed_origins(
            simple_origin_config_builder(),
            simple_origin_request_builder,
        )
    }

    fn simple_origin_config_builder() -> CorsBuilder {
        CorsBuilder::new()
    }

    fn simple_origin_request_builder() -> TestResult<http::request::Builder> {
        Ok(http::Request::builder())
    }

    #[test]
    fn simple_response_includes_vary_header() -> TestResult {
        let builder = CorsBuilder::new()
            .allow_origins(AllowedOrigins::Any { allow_null: false })
            .allow_methods(vec![Method::POST]);

        let req = http::Request::builder()
            .header(
                header::ORIGIN,
                HeaderValue::from_static("http://test.example"),
            )
            .body(())?;

        common_test_vary_header(builder, req, CorsResource::into_simple)
    }

    #[test]
    fn simple_response_includes_allowed_credentials() -> TestResult {
        let builder = CorsBuilder::new()
            .allow_origins(AllowedOrigins::Any { allow_null: false })
            .allow_methods(vec![Method::POST]);

        let req = http::Request::builder()
            .header(
                header::ORIGIN,
                HeaderValue::from_static("http://test.example"),
            )
            .body(())?;

        common_test_allowed_credentials(builder, req, CorsResource::into_simple)
    }

    #[test]
    fn simple_response_includes_allowed_origin() -> TestResult {
        let builder = CorsBuilder::new()
            .allow_origins(AllowedOrigins::Any { allow_null: false })
            .allow_methods(vec![Method::POST]);

        let req = http::Request::builder()
            .header(
                header::ORIGIN,
                HeaderValue::from_static("http://test.example"),
            )
            .body(())?;

        common_test_allowed_origin(builder, req, CorsResource::into_simple)
    }

    #[test]
    fn simple_response_includes_exposed_headers() -> TestResult {
        let cfg = CorsBuilder::new()
            .allow_origins(AllowedOrigins::Any { allow_null: false })
            .allow_methods(vec![Method::POST])
            .expose_headers(&[header::WARNING, HeaderName::from_static("x-custom")])
            .into_config();

        let req = http::Request::builder()
            .header(
                header::ORIGIN,
                HeaderValue::from_static("http://test.example"),
            )
            .body(())?;

        let mut headers = cfg.process_request(&req)?.into_simple()?;
        let hdr = headers
            .remove(header::ACCESS_CONTROL_EXPOSE_HEADERS)
            .expect("expose-headers header missing");

        assert_set!(hdr, "warning", "x-custom");

        Ok(())
    }

    #[test]
    fn preflight_allows_when_origin_is_any() -> TestResult {
        common_allows_when_origin_is_any(
            preflight_origin_config_builder(),
            preflight_origin_request_builder,
        )
    }

    #[test]
    fn preflight_disallows_null_origin_even_for_any() -> TestResult {
        common_disallows_null_origin_even_for_any(
            preflight_origin_config_builder(),
            preflight_origin_request_builder,
        )
    }

    #[test]
    fn preflight_allows_null_origin_for_any_when_configured() -> TestResult {
        common_allows_null_origin_for_any_when_configured(
            preflight_origin_config_builder(),
            preflight_origin_request_builder,
        )
    }

    #[test]
    fn preflight_compares_origin_against_allowed_origins() -> TestResult {
        common_compares_origin_against_allowed_origins(
            preflight_origin_config_builder(),
            preflight_origin_request_builder,
        )
    }

    fn preflight_origin_config_builder() -> CorsBuilder {
        CorsBuilder::new().allow_methods(vec![Method::POST])
    }

    fn preflight_origin_request_builder() -> TestResult<http::request::Builder> {
        let builder = http::Request::builder();
        let builder = builder.method(Method::OPTIONS).header(
            header::ACCESS_CONTROL_REQUEST_METHOD,
            HeaderValue::from_static("POST"),
        );
        Ok(builder)
    }

    #[test]
    fn preflight_compares_method_against_allowed_methods() -> TestResult {
        let cfg = CorsBuilder::new()
            .allow_origins(AllowedOrigins::Any { allow_null: false })
            .allow_methods(vec![Method::POST, Method::PATCH])
            .into_config();

        let builder = || -> TestResult<http::request::Builder> {
            let builder = http::Request::builder();
            let builder = builder.method(Method::OPTIONS).header(
                header::ORIGIN,
                HeaderValue::from_static("http://test.example"),
            );
            Ok(builder)
        };

        let allowed_req_post = builder()?
            .header(
                header::ACCESS_CONTROL_REQUEST_METHOD,
                HeaderValue::from_static("POST"),
            )
            .body(())?;

        assert_variant!(cfg.process_request(&allowed_req_post), Ok(_));

        let allowed_req_patch = builder()?
            .header(
                header::ACCESS_CONTROL_REQUEST_METHOD,
                HeaderValue::from_static("PATCH"),
            )
            .body(())?;

        assert_variant!(cfg.process_request(&allowed_req_patch), Ok(_));

        let disallowed_req_put = builder()?
            .header(
                header::ACCESS_CONTROL_REQUEST_METHOD,
                HeaderValue::from_static("PUT"),
            )
            .body(())?;

        assert_variant!(
            cfg.process_request(&disallowed_req_put),
            Err(_disallowed_method)
        );

        Ok(())
    }

    #[test]
    fn preflight_compares_headers_against_allowed_headers() -> TestResult {
        let cfg = CorsBuilder::new()
            .allow_origins(AllowedOrigins::Any { allow_null: false })
            .allow_methods(vec![Method::POST])
            .allow_headers(&[
                header::SERVER,
                header::WARNING,
                HeaderName::from_static("x-custom"),
            ])
            .into_config();

        let builder = || -> TestResult<http::request::Builder> {
            let builder = http::Request::builder();
            let builder = builder
                .method(Method::OPTIONS)
                .header(
                    header::ORIGIN,
                    HeaderValue::from_static("http://test.example"),
                )
                .header(
                    header::ACCESS_CONTROL_REQUEST_METHOD,
                    HeaderValue::from_static("POST"),
                );
            Ok(builder)
        };

        let allowed_req_server = builder()?
            .header(
                header::ACCESS_CONTROL_REQUEST_HEADERS,
                HeaderValue::from(header::SERVER),
            )
            .body(())?;

        assert_variant!(cfg.process_request(&allowed_req_server), Ok(_));

        let allowed_req_warning = builder()?
            .header(
                header::ACCESS_CONTROL_REQUEST_HEADERS,
                HeaderValue::from(header::WARNING),
            )
            .body(())?;

        assert_variant!(cfg.process_request(&allowed_req_warning), Ok(_));

        let allowed_req_multiple = builder()?
            .header(
                header::ACCESS_CONTROL_REQUEST_HEADERS,
                HeaderValue::from_static("server,warning,x-custom"),
            )
            .body(())?;

        assert_variant!(cfg.process_request(&allowed_req_multiple), Ok(_));

        let allowed_req_differing_case = builder()?
            .header(
                header::ACCESS_CONTROL_REQUEST_HEADERS,
                HeaderValue::from_static("Server,WARNING,X-cUsToM"),
            )
            .body(())?;

        assert_variant!(cfg.process_request(&allowed_req_differing_case), Ok(_));

        let disallowed_req_range = builder()?
            .header(
                header::ACCESS_CONTROL_REQUEST_HEADERS,
                HeaderValue::from(header::CONTENT_RANGE),
            )
            .body(())?;

        assert_variant!(
            cfg.process_request(&disallowed_req_range),
            Err(_disallowed_header)
        );

        Ok(())
    }

    #[test]
    fn preflight_response_includes_vary_header() -> TestResult {
        let builder = CorsBuilder::new()
            .allow_origins(AllowedOrigins::Any { allow_null: false })
            .allow_methods(vec![Method::POST]);

        let req = http::Request::builder()
            .method(Method::OPTIONS)
            .header(
                header::ORIGIN,
                HeaderValue::from_static("http://test.example"),
            )
            .header(
                header::ACCESS_CONTROL_REQUEST_METHOD,
                HeaderValue::from_static("POST"),
            )
            .body(())?;

        common_test_vary_header(builder, req, CorsResource::into_preflight)
    }

    #[test]
    fn preflight_response_includes_allowed_credentials() -> TestResult {
        let builder = CorsBuilder::new()
            .allow_origins(AllowedOrigins::Any { allow_null: false })
            .allow_methods(vec![Method::POST]);

        let req = http::Request::builder()
            .method(Method::OPTIONS)
            .header(
                header::ORIGIN,
                HeaderValue::from_static("http://test.example"),
            )
            .header(
                header::ACCESS_CONTROL_REQUEST_METHOD,
                HeaderValue::from_static("POST"),
            )
            .body(())?;

        common_test_allowed_credentials(builder, req, CorsResource::into_preflight)
    }

    #[test]
    fn preflight_response_includes_allowed_origin() -> TestResult {
        let builder = CorsBuilder::new()
            .allow_origins(AllowedOrigins::Any { allow_null: false })
            .allow_methods(vec![Method::POST]);

        let req = http::Request::builder()
            .method(Method::OPTIONS)
            .header(
                header::ORIGIN,
                HeaderValue::from_static("http://test.example"),
            )
            .header(
                header::ACCESS_CONTROL_REQUEST_METHOD,
                HeaderValue::from_static("POST"),
            )
            .body(())?;

        common_test_allowed_origin(builder, req, CorsResource::into_preflight)
    }

    #[test]
    fn preflight_response_includes_allowed_methods() -> TestResult {
        let cfg = CorsBuilder::new()
            .allow_origins(AllowedOrigins::Any { allow_null: false })
            .allow_methods(vec![
                Method::POST,
                Method::PATCH,
                Method::from_bytes(b"LIST")?,
            ])
            .into_config();

        let req = http::Request::builder()
            .method(Method::OPTIONS)
            .header(
                header::ORIGIN,
                HeaderValue::from_static("http://test.example"),
            )
            .header(
                header::ACCESS_CONTROL_REQUEST_METHOD,
                HeaderValue::from_static("POST"),
            )
            .body(())?;

        let mut headers = cfg.process_request(&req)?.into_preflight()?;
        let hdr = headers
            .remove(header::ACCESS_CONTROL_ALLOW_METHODS)
            .expect("allow-methods header missing");

        assert_set!(hdr, "PATCH", "LIST", "POST");

        Ok(())
    }

    #[test]
    fn preflight_response_includes_allowed_headers() -> TestResult {
        let cfg = CorsBuilder::new()
            .allow_origins(AllowedOrigins::Any { allow_null: false })
            .allow_methods(vec![Method::POST])
            .allow_headers(&[
                header::SERVER,
                header::WARNING,
                HeaderName::from_static("x-custom"),
            ])
            .into_config();

        let req = http::Request::builder()
            .method(Method::OPTIONS)
            .header(
                header::ORIGIN,
                HeaderValue::from_static("http://test.example"),
            )
            .header(
                header::ACCESS_CONTROL_REQUEST_METHOD,
                HeaderValue::from_static("POST"),
            )
            .body(())?;

        let mut headers = cfg.process_request(&req)?.into_preflight()?;
        let hdr = headers
            .remove(header::ACCESS_CONTROL_ALLOW_HEADERS)
            .expect("allow-headers header missing");

        assert_set!(hdr, "server", "warning", "x-custom");

        Ok(())
    }

    #[test]
    fn preflight_response_includes_max_age() -> TestResult {
        let cfg = CorsBuilder::new()
            .allow_origins(AllowedOrigins::Any { allow_null: false })
            .allow_methods(vec![Method::POST])
            .max_age(Duration::from_secs(42))
            .into_config();

        let req = http::Request::builder()
            .method(Method::OPTIONS)
            .header(
                header::ORIGIN,
                HeaderValue::from_static("http://test.example"),
            )
            .header(
                header::ACCESS_CONTROL_REQUEST_METHOD,
                HeaderValue::from_static("POST"),
            )
            .body(())?;

        let mut headers = cfg.process_request(&req)?.into_preflight()?;
        let hdr = headers
            .remove(header::ACCESS_CONTROL_MAX_AGE)
            .expect("max-age header missing");

        assert_eq!(hdr, "42");

        Ok(())
    }

    fn common_allows_when_origin_is_any(
        cfg_builder: CorsBuilder,
        req_builder: impl Fn() -> TestResult<http::request::Builder>,
    ) -> TestResult {
        let cfg = cfg_builder
            .allow_origins(AllowedOrigins::Any { allow_null: false })
            .into_config();

        let req = req_builder()?
            .header(
                header::ORIGIN,
                HeaderValue::from_static("http://test.example"),
            )
            .body(())?;

        assert_variant!(cfg.process_request(&req), Ok(_));

        Ok(())
    }

    fn common_disallows_null_origin_even_for_any(
        cfg_builder: CorsBuilder,
        req_builder: impl Fn() -> TestResult<http::request::Builder>,
    ) -> TestResult {
        let cfg = cfg_builder
            .allow_origins(AllowedOrigins::Any { allow_null: false })
            .into_config();

        let req = req_builder()?
            .header(header::ORIGIN, HeaderValue::from_static("null"))
            .body(())?;

        assert_variant!(cfg.process_request(&req), Err(_disallowed_origin));

        Ok(())
    }

    fn common_allows_null_origin_for_any_when_configured(
        cfg_builder: CorsBuilder,
        req_builder: impl Fn() -> TestResult<http::request::Builder>,
    ) -> TestResult {
        let cfg = cfg_builder
            .allow_origins(AllowedOrigins::Any { allow_null: true })
            .into_config();

        let req = req_builder()?
            .header(header::ORIGIN, HeaderValue::from_static("null"))
            .body(())?;

        assert_variant!(cfg.process_request(&req), Ok(_));

        Ok(())
    }

    fn common_compares_origin_against_allowed_origins(
        cfg_builder: CorsBuilder,
        req_builder: impl Fn() -> TestResult<http::request::Builder>,
    ) -> TestResult {
        let cfg = cfg_builder
            .allow_origins(AllowedOrigins::from_iter(vec![
                HeaderValue::from_static("http://foo.example"),
                HeaderValue::from_static("http://bar.example"),
            ]))
            .into_config();

        let allowed_req_foo = req_builder()?
            .header(
                header::ORIGIN,
                HeaderValue::from_static("http://foo.example"),
            )
            .body(())?;

        assert_variant!(cfg.process_request(&allowed_req_foo), Ok(_));

        let allowed_req_bar = req_builder()?
            .header(
                header::ORIGIN,
                HeaderValue::from_static("http://bar.example"),
            )
            .body(())?;

        assert_variant!(cfg.process_request(&allowed_req_bar), Ok(_));

        let disallowed_req_unlisted = req_builder()?
            .header(
                header::ORIGIN,
                HeaderValue::from_static("http://quux.example"),
            )
            .body(())?;

        assert_variant!(
            cfg.process_request(&disallowed_req_unlisted),
            Err(_disallowed_origin)
        );

        let disallowed_req_differing_case = req_builder()?
            .header(
                header::ORIGIN,
                HeaderValue::from_static("http://FOO.example"),
            )
            .body(())?;

        assert_variant!(
            cfg.process_request(&disallowed_req_differing_case),
            Err(_disallowed_origin)
        );

        let disallowed_req_differing_scheme = req_builder()?
            .header(
                header::ORIGIN,
                HeaderValue::from_static("https://foo.example"),
            )
            .body(())?;

        assert_variant!(
            cfg.process_request(&disallowed_req_differing_scheme),
            Err(_disallowed_origin)
        );

        Ok(())
    }

    fn common_test_vary_header<B>(
        builder: CorsBuilder,
        req: http::Request<B>,
        f: impl Fn(CorsResource) -> TestResult<HeaderMap>,
    ) -> TestResult {
        let cfg = builder.into_config();

        let mut headers = f(cfg.process_request(&req)?)?;

        let hdr = headers.remove(header::VARY).expect("vary header missing");

        assert_set!(
            hdr,
            "origin",
            "access-control-request-method",
            "access-control-request-headers"
        );

        Ok(())
    }

    fn common_test_allowed_credentials<B>(
        builder: CorsBuilder,
        req: http::Request<B>,
        f: impl Fn(CorsResource) -> TestResult<HeaderMap>,
    ) -> TestResult {
        let cfg = builder.allow_credentials(true).into_config();

        let mut headers = f(cfg.process_request(&req)?)?;
        let hdr = headers
            .remove(header::ACCESS_CONTROL_ALLOW_CREDENTIALS)
            .expect("allow-credentials header missing");

        assert_eq!(hdr, "true");

        Ok(())
    }

    fn common_test_allowed_origin<B>(
        builder: CorsBuilder,
        req: http::Request<B>,
        f: impl Fn(CorsResource) -> TestResult<HeaderMap>,
    ) -> TestResult {
        let cfg_no_wildcard_no_credentials = builder.clone().into_config();
        let mut headers = f(cfg_no_wildcard_no_credentials.process_request(&req)?)?;
        let hdr = headers
            .remove(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .expect("allow-origin header missing");

        assert_eq!(hdr, "http://test.example");

        let cfg_wildcard_no_credentials = builder.clone().prefer_wildcard(true).into_config();
        let mut headers = f(cfg_wildcard_no_credentials.process_request(&req)?)?;
        let hdr = headers
            .remove(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .expect("allow-origin header missing");

        assert_eq!(hdr, "*");

        let cfg_wildcard_credentials = builder
            .clone()
            .prefer_wildcard(true)
            .allow_credentials(true)
            .into_config();
        let mut headers = f(cfg_wildcard_credentials.process_request(&req)?)?;
        let hdr = headers
            .remove(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .expect("allow-origin header missing");

        assert_eq!(hdr, "http://test.example");

        Ok(())
    }
}
