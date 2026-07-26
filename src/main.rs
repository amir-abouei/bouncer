use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full, Limited};
use hyper::body::{Body, Incoming};
use hyper::header::{HeaderName, HeaderValue};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode, Uri};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioIo;
use once_cell::sync::Lazy;
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use bouncer::allowlist::{self, SharedAllowlist};
use bouncer::auth;
use bouncer::routing;

type BoxError = Box<dyn std::error::Error + Send + Sync>;
type ProxyBody = BoxBody<Bytes, BoxError>;
type ProxyClient = Client<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>, ProxyBody>;

static CLIENT: Lazy<ProxyClient> = Lazy::new(|| {
    let https = HttpsConnectorBuilder::new()
        .with_webpki_roots()
        .https_or_http()
        .enable_http1()
        .build();
    Client::builder(hyper_util::rt::TokioExecutor::new())
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .pool_max_idle_per_host(32)
        .build(https)
});

const MAX_BODY_BYTES: usize = 100 * 1024 * 1024; // 100 MB

const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "host",
];

fn box_body<B>(body: B) -> ProxyBody
where
    B: Body<Data = Bytes> + Send + Sync + 'static,
    B::Error: Into<BoxError>,
{
    body.map_err(Into::into).boxed()
}

fn plain_response(status: StatusCode, msg: &'static str) -> Response<ProxyBody> {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain")
        .body(box_body(Full::new(Bytes::from_static(msg.as_bytes()))))
        .unwrap()
}

fn unauthorized_response() -> Response<ProxyBody> {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header("content-type", "text/plain")
        .body(box_body(Full::new(Bytes::from_static(b"Unauthorized\n"))))
        .unwrap()
}

fn is_hop_by_hop(name: &HeaderName) -> bool {
    HOP_BY_HOP.contains(&name.as_str())
}

async fn handle(
    req: Request<Incoming>,
    allowlist: SharedAllowlist,
) -> Result<Response<ProxyBody>, Infallible> {
    let (parts, body) = req.into_parts();
    let path = parts.uri.path();

    if path == "/healthz" {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/plain")
            .body(box_body(Full::new(Bytes::from_static(b"ok\n"))))
            .unwrap());
    }

    let host_header = parts
        .headers
        .get(routing::HOST_HEADER_NAME)
        .and_then(|v| v.to_str().ok());

    let Some((alias, rest)) = routing::extract_alias(path, host_header) else {
        return Ok(plain_response(
            StatusCode::NOT_FOUND,
            "Specify a proxy target, e.g. /telegram/<path> or X-Bouncer-Host: telegram\n",
        ));
    };

    let Some(target) = allowlist::resolve(&allowlist, alias) else {
        tracing::warn!(alias, "rejected: alias not in allowlist");
        return Ok(plain_response(StatusCode::FORBIDDEN, "Unknown proxy target\n"));
    };

    if target.auth {
        if let Err(e) = auth::authorize(&parts.headers) {
            tracing::warn!(alias, error = %e, "rejected: unauthorized");
            return Ok(unauthorized_response());
        }
    }

    let upstream_host = target.host;

    let mut upstream_url = String::with_capacity(8 + upstream_host.len() + rest.len() + 1);
    upstream_url.push_str("https://");
    upstream_url.push_str(&upstream_host);
    if !rest.is_empty() {
        upstream_url.push('/');
        upstream_url.push_str(rest);
    }
    if let Some(q) = parts.uri.query() {
        upstream_url.push('?');
        upstream_url.push_str(q);
    }

    let upstream_uri: Uri = match upstream_url.parse() {
        Ok(u) => u,
        Err(e) => {
            tracing::error!(error = %e, path = %path, "failed to build upstream URI");
            return Ok(plain_response(StatusCode::BAD_GATEWAY, "Invalid upstream URI\n"));
        }
    };

    let limited_body = Limited::new(body, MAX_BODY_BYTES);

    let mut upstream_req_builder = Request::builder().method(parts.method).uri(upstream_uri);

    {
        let out_headers = upstream_req_builder.headers_mut().unwrap();
        for (name, value) in parts.headers.iter() {
            if is_hop_by_hop(name)
                || name.as_str() == auth::HEADER_NAME
                || name.as_str() == routing::HOST_HEADER_NAME
            {
                continue;
            }
            out_headers.insert(name.clone(), value.clone());
        }
        out_headers.insert(
            HeaderName::from_static("host"),
            HeaderValue::from_str(&upstream_host).unwrap(),
        );
    }

    let upstream_req = match upstream_req_builder.body(box_body(limited_body)) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "failed to build upstream request");
            return Ok(plain_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to build upstream request\n",
            ));
        }
    };

    match CLIENT.request(upstream_req).await {
        Ok(upstream_resp) => {
            let (resp_parts, resp_body) = upstream_resp.into_parts();

            let mut resp_builder = Response::builder().status(resp_parts.status);
            {
                let out_headers = resp_builder.headers_mut().unwrap();
                for (name, value) in resp_parts.headers.iter() {
                    if is_hop_by_hop(name) {
                        continue;
                    }
                    out_headers.insert(name.clone(), value.clone());
                }
            }

            let limited_resp_body = Limited::new(resp_body, MAX_BODY_BYTES);

            Ok(resp_builder
                .body(box_body(limited_resp_body))
                .unwrap_or_else(|_| {
                    plain_response(StatusCode::INTERNAL_SERVER_ERROR, "Response build failed\n")
                }))
        }
        Err(e) => {
            tracing::error!(error = %e, upstream = %upstream_host, "upstream request failed");
            Ok(plain_response(StatusCode::BAD_GATEWAY, "Upstream request failed\n"))
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let jwt_secret_configured = auth::secret_configured();

    let config_path =
        std::env::var("ALLOWLIST_PATH").unwrap_or_else(|_| "config/allowlist.toml".to_string());
    let allowlist = allowlist::load_and_watch(&config_path, jwt_secret_configured)?;

    let addr: SocketAddr = std::env::var("LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse()?;

    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "bouncer listening");

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let allowlist = allowlist.clone();

        tokio::spawn(async move {
            let service = service_fn(move |req| handle(req, allowlist.clone()));
            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                tracing::debug!(error = %err, "connection error");
            }
        });
    }
}
