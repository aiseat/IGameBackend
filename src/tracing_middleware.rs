use actix_web::dev::{Payload, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{Error, FromRequest, HttpMessage, HttpRequest, ResponseError};
use futures::future::{ok, ready, Ready};
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;
use tracing::Span;
use tracing_actix_web::{root_span_macro::private, RootSpanBuilder};
use tracing_futures::Instrument;

/// We will define a custom root span builder to capture additional fields, specific
/// to our application, on top of the ones provided by `DefaultRootSpanBuilder` out of the box.
pub struct CustomRootSpanBuilder;

impl RootSpanBuilder for CustomRootSpanBuilder {
    fn on_request_start(request: &ServiceRequest) -> Span {
        let connection_info = request.connection_info();
        let http_route: std::borrow::Cow<'static, str> = request
            .match_pattern()
            .map(Into::into)
            .unwrap_or_else(|| "default".into());
        let user_agent = request
            .headers()
            .get("User-Agent")
            .map(|h| h.to_str().unwrap_or(""))
            .unwrap_or("");
        let span = private::tracing::info_span!(
            "HTTP request",
            id = %private::get_request_id(request),
            ip = %connection_info.realip_remote_addr().unwrap_or(""),
            host = %connection_info.host(),
            method = %private::http_method_str(request.method()),
            target = %request.uri().path_and_query().map(|p| p.as_str()).unwrap_or(""),
            route = %http_route,
            status_code = private::tracing::field::Empty,
            user_agent = %user_agent,
            exception = private::tracing::field::Empty,
        );
        std::mem::drop(connection_info);
        span
    }

    fn on_request_end<B>(span: Span, outcome: &Result<ServiceResponse<B>, Error>) {
        match &outcome {
            Ok(response) => {
                if let Some(error) = response.response().error() {
                    handle_error(span, error)
                } else {
                    span.record("status_code", &response.response().status().as_u16());
                }
            }
            Err(error) => handle_error(span, error),
        };
    }
}

fn handle_error(span: Span, error: &actix_web::Error) {
    let response_error = error.as_response_error();
    let status_code = response_error.status_code();
    span.record("status_code", &status_code.as_u16());
    span.record("exception", &tracing::field::display(response_error));
}

pub struct TracingLogger<RootSpan: RootSpanBuilder> {
    root_span_builder: std::marker::PhantomData<RootSpan>,
}

impl<RootSpan: RootSpanBuilder> Clone for TracingLogger<RootSpan> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<RootSpan: RootSpanBuilder> TracingLogger<RootSpan> {
    pub fn new() -> TracingLogger<RootSpan> {
        TracingLogger {
            root_span_builder: Default::default(),
        }
    }
}

impl<S, B, RootSpan> Transform<S, ServiceRequest> for TracingLogger<RootSpan>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
    RootSpan: RootSpanBuilder,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = TracingLoggerMiddleware<S, RootSpan>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(TracingLoggerMiddleware {
            service,
            root_span_builder: std::marker::PhantomData::default(),
        })
    }
}

#[doc(hidden)]
pub struct TracingLoggerMiddleware<S, RootSpanBuilder> {
    service: S,
    root_span_builder: std::marker::PhantomData<RootSpanBuilder>,
}

impl<S, B, RootSpanType> Service<ServiceRequest> for TracingLoggerMiddleware<S, RootSpanType>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
    RootSpanType: RootSpanBuilder,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start_count = Instant::now();
        req.extensions_mut().insert(private::generate_request_id());
        let root_span = RootSpanType::on_request_start(&req);
        let root_span_wrapper = RootSpan::new(root_span.clone());
        req.extensions_mut().insert(root_span_wrapper);

        tracing::debug!("????????????: {:?}", req);
        let fut = self.service.call(req);
        Box::pin(
            async move {
                let outcome = fut.await;
                RootSpanType::on_request_end(Span::current(), &outcome);
                emit_event_on_error(&outcome);
                tracing::info!("???????????????????????????: {:?}", start_count.elapsed(),);
                outcome
            }
            .instrument(root_span),
        )
    }
}

fn emit_event_on_error<B: 'static>(outcome: &Result<ServiceResponse<B>, actix_web::Error>) {
    match outcome {
        Ok(response) => {
            if let Some(err) = response.response().error() {
                emit_error_event(err.as_response_error())
            }
        }
        Err(error) => {
            let response_error = error.as_response_error();
            emit_error_event(response_error)
        }
    }
}

fn emit_error_event(response_error: &dyn ResponseError) {
    let status_code = response_error.status_code();
    if status_code.is_client_error() {
        tracing::debug!("??????http????????????: ???????????????");
    } else {
        tracing::error!("??????http????????????: ???????????????");
    }
}

#[derive(Clone)]
pub struct RootSpan(Span);

impl RootSpan {
    pub(crate) fn new(span: Span) -> Self {
        Self(span)
    }
}

impl std::ops::Deref for RootSpan {
    type Target = Span;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::convert::Into<Span> for RootSpan {
    fn into(self) -> Span {
        self.0
    }
}

impl FromRequest for RootSpan {
    type Error = RootSpanExtractionError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ready(
            req.extensions()
                .get::<RootSpan>()
                .cloned()
                .ok_or(RootSpanExtractionError { _priv: () }),
        )
    }
}

#[derive(Debug)]
pub struct RootSpanExtractionError {
    _priv: (),
}

impl ResponseError for RootSpanExtractionError {}

impl std::fmt::Display for RootSpanExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failed to retrieve the root span from request-local storage."
        )
    }
}

impl std::error::Error for RootSpanExtractionError {}
