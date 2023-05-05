use crate::{
    config::{NetworkConfig, PeerConfig, ServerConfig},
    helpers::{HelperIdentity, TransportCallbacks},
    net::{HttpTransport, MpcHelperClient, MpcHelperServer},
    sync::Arc,
    test_fixture::metrics::MetricsHandle,
};
use axum::{
    body::{Body, Bytes},
    extract::{BodyStream, FromRequest, RequestParts},
    http::Request,
};
use futures::Stream;
use hyper::{
    client::HttpConnector,
    http::{uri::Scheme, Uri},
};
use hyper_tls::{native_tls::TlsConnector, HttpsConnector};
use once_cell::sync::Lazy;
use std::{array, error::Error as StdError, net::SocketAddr, ops::Deref};
use tokio::task::JoinHandle;

static DEFAULT_CLIENT_CONFIG: Lazy<PeerConfig> =
    Lazy::new(|| PeerConfig::new("http://localhost:3000".parse().unwrap()));

type HttpTransportCallbacks = TransportCallbacks<Arc<HttpTransport>>;

pub async fn body_stream(
    stream: Box<dyn Stream<Item = Result<Bytes, Box<dyn StdError + Send + Sync>>> + Send>,
) -> BodyStream {
    BodyStream::from_request(&mut RequestParts::new(
        Request::builder()
            .uri("/ignored")
            .body(Body::from(stream))
            .unwrap(),
    ))
    .await
    .unwrap()
}

pub struct TestServer {
    pub addr: SocketAddr,
    pub handle: JoinHandle<()>,
    pub transport: Arc<HttpTransport>,
    pub server: MpcHelperServer,
    pub client: MpcHelperClient,
}

impl TestServer {
    /// Build default set of test clients
    ///
    /// All three clients will be configured with the same default server URL, thus,
    /// at most one client will do anything useful.
    pub async fn default() -> TestServer {
        Self::builder().build().await
    }

    /// Return a test client builder
    pub fn builder() -> TestServerBuilder {
        TestServerBuilder::default()
    }
}

#[derive(Default)]
pub struct TestServerBuilder {
    callbacks: Option<HttpTransportCallbacks>,
    metrics: Option<MetricsHandle>,
    https: bool,
}

/// Construct an *insecure* HTTPS client for a test server.
///
/// The resulting client accepts invalid server certificates and is thus only suitable for test
/// usage.
fn https_client(addr: SocketAddr) -> MpcHelperClient {
    // requires custom client to use self signed certs
    let conn = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    let mut http = HttpConnector::new();
    http.enforce_http(false);
    let https = HttpsConnector::<HttpConnector>::from((http, conn.into()));
    let uri = Uri::builder()
        .scheme(Scheme::HTTPS)
        .authority(format!("localhost:{}", addr.port()))
        .path_and_query("/")
        .build()
        .unwrap();
    MpcHelperClient::new_with_connector(uri, https)
}

impl TestServerBuilder {
    pub fn with_callbacks(mut self, callbacks: HttpTransportCallbacks) -> Self {
        self.callbacks = Some(callbacks);
        self
    }

    #[cfg(all(test, feature = "in-memory-infra"))] // only used in unit tests
    pub fn with_metrics(mut self, metrics: MetricsHandle) -> Self {
        self.metrics = Some(metrics);
        self
    }

    #[allow(dead_code)] // TODO: fix when TLS is enabled
    pub fn https(mut self) -> Self {
        self.https = true;
        self
    }

    pub async fn build(self) -> TestServer {
        let server_config = if self.https {
            ServerConfig::https_self_signed()
        } else {
            ServerConfig::http()
        };
        let clients = TestClients::default();
        let (transport, server) = HttpTransport::new(
            HelperIdentity::ONE,
            server_config,
            clients.into(),
            self.callbacks.unwrap_or_default(),
        );
        let (addr, handle) = server.start(self.metrics).await;
        let client = if self.https {
            https_client(addr)
        } else {
            MpcHelperClient::with_str_addr(&format!("http://{addr}")).unwrap()
        };
        TestServer {
            addr,
            handle,
            transport,
            server,
            client,
        }
    }
}

pub struct TestClients(pub [MpcHelperClient; 3]);

impl Deref for TestClients {
    type Target = [MpcHelperClient; 3];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<TestClients> for [MpcHelperClient; 3] {
    fn from(value: TestClients) -> [MpcHelperClient; 3] {
        value.0
    }
}

impl TestClients {
    /// Build default set of test clients
    ///
    /// All three clients will be configured with the same default server URL, thus,
    /// at most one client will do anything useful.
    pub fn default() -> Self {
        Self::builder().build()
    }

    /// Return a test client builder
    pub fn builder() -> TestClientsBuilder {
        TestClientsBuilder::default()
    }
}

#[derive(Default)]
pub struct TestClientsBuilder {
    network_config: Option<NetworkConfig>,
}

impl TestClientsBuilder {
    pub fn with_network_config(mut self, network_config: NetworkConfig) -> Self {
        self.network_config = Some(network_config);
        self
    }

    pub fn build(self) -> TestClients {
        TestClients(match self.network_config {
            Some(config) => MpcHelperClient::from_conf(&config),
            None => array::from_fn(|_| MpcHelperClient::new(DEFAULT_CLIENT_CONFIG.clone())),
        })
    }
}
