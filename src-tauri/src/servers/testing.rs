//! Сервер для тестов: HTTP/1.1 или HTTPS (тестовый CA, сертификат на `localhost` и `127.0.0.1`)
//! на локальном порту, ответы по пути запроса. Сеть за пределами машины тесты не трогают.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;

/// CA, которым подписан сертификат тестового HTTPS-сервера.
pub const TEST_CA_PEM: &str = include_str!("testdata/ca.pem");
const SERVER_CERT_PEM: &str = include_str!("testdata/server.pem");
const SERVER_KEY_PEM: &str = include_str!("testdata/server-key.pem");

#[derive(Clone, Debug)]
pub struct Reply {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
    pub delay: Option<Duration>,
    pub headers: Vec<(&'static str, String)>,
}

impl Reply {
    pub fn json(body: &str) -> Self {
        Self::with("application/json", 200, body)
    }

    pub fn text(body: &str) -> Self {
        Self::with("text/plain; version=0.0.4", 200, body)
    }

    pub fn status(status: u16) -> Self {
        Self::with("text/plain", status, "")
    }

    fn with(content_type: &'static str, status: u16, body: &str) -> Self {
        Self {
            status,
            content_type,
            body: body.as_bytes().to_vec(),
            delay: None,
            headers: Vec::new(),
        }
    }

    pub fn delayed(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    pub fn header(mut self, name: &'static str, value: &str) -> Self {
        self.headers.push((name, value.to_string()));
        self
    }
}

/// Запрос, который получил тестовый сервер.
#[derive(Clone, Debug, PartialEq)]
pub struct Seen {
    pub path: String,
    pub authorization: Option<String>,
    pub user_agent: Option<String>,
}

pub struct TestServer {
    pub addr: SocketAddr,
    pub tls: bool,
    pub seen: Arc<Mutex<Vec<Seen>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl TestServer {
    pub async fn start(routes: Vec<(&str, Reply)>) -> Self {
        Self::spawn(routes, false).await
    }

    pub async fn start_tls(routes: Vec<(&str, Reply)>) -> Self {
        Self::spawn(routes, true).await
    }

    /// Адрес сервера: `http://127.0.0.1:<порт>` или `https://127.0.0.1:<порт>`.
    pub fn url(&self) -> String {
        let scheme = if self.tls { "https" } else { "http" };
        format!("{scheme}://{}", self.addr)
    }

    pub fn paths(&self) -> Vec<String> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|s| s.path.clone())
            .collect()
    }

    async fn spawn(routes: Vec<(&str, Reply)>, tls: bool) -> Self {
        crate::tls::ensure_crypto_provider();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let routes: Arc<HashMap<String, Reply>> = Arc::new(
            routes
                .into_iter()
                .map(|(p, r)| (p.to_string(), r))
                .collect(),
        );
        let seen = Arc::new(Mutex::new(Vec::new()));
        let acceptor = tls.then(acceptor);
        let task = {
            let seen = seen.clone();
            tokio::spawn(async move {
                loop {
                    let Ok((stream, _)) = listener.accept().await else {
                        return;
                    };
                    let (routes, seen, acceptor) = (routes.clone(), seen.clone(), acceptor.clone());
                    tokio::spawn(async move {
                        match acceptor {
                            Some(a) => {
                                if let Ok(s) = a.accept(stream).await {
                                    serve(s, &routes, &seen).await;
                                }
                            }
                            None => serve(stream, &routes, &seen).await,
                        }
                    });
                }
            })
        };
        Self {
            addr,
            tls,
            seen,
            task,
        }
    }
}

fn acceptor() -> tokio_rustls::TlsAcceptor {
    use rustls::pki_types::pem::PemObject;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    let certs = CertificateDer::pem_slice_iter(SERVER_CERT_PEM.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let key = PrivateKeyDer::from_pem_slice(SERVER_KEY_PEM.as_bytes()).unwrap();
    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    tokio_rustls::TlsAcceptor::from(Arc::new(config))
}

async fn serve<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    routes: &HashMap<String, Reply>,
    seen: &Mutex<Vec<Seen>>,
) {
    let mut head = Vec::new();
    let mut buf = [0u8; 4096];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(n) => head.extend_from_slice(&buf[..n]),
        }
        // Запись TLS (ClientHello начинается с байта 0x16) пришла на сервер без TLS, то есть в
        // профиле `https://`, а сервер работает по HTTP. Обычные HTTP-серверы отвечают на такой
        // мусор текстом «400 Bad Request»; клиент TLS принимает этот ответ за повреждённую запись.
        if head.first() == Some(&0x16) {
            let _ = stream
                .write_all(
                    b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await;
            let _ = stream.shutdown().await;
            return;
        }
        if head.len() > 64 * 1024 {
            return;
        }
    }
    let text = String::from_utf8_lossy(&head);
    let mut lines = text.lines();
    let path = lines
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("")
        .to_string();
    let header = |name: &str| {
        text.lines().skip(1).find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.trim()
                .eq_ignore_ascii_case(name)
                .then(|| v.trim().to_string())
        })
    };
    seen.lock().unwrap().push(Seen {
        path: path.clone(),
        authorization: header("authorization"),
        user_agent: header("user-agent"),
    });
    let reply = routes
        .get(&path)
        .cloned()
        .unwrap_or_else(|| Reply::status(404));
    if let Some(d) = reply.delay {
        tokio::time::sleep(d).await;
    }
    let mut out = format!(
        "HTTP/1.1 {} X\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        reply.status,
        reply.content_type,
        reply.body.len()
    );
    for (k, v) in &reply.headers {
        out.push_str(&format!("{k}: {v}\r\n"));
    }
    out.push_str("\r\n");
    let _ = stream.write_all(out.as_bytes()).await;
    let _ = stream.write_all(&reply.body).await;
    let _ = stream.shutdown().await;
}
