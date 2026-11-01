//! Источник карт GPU (спека I1d §4): экспортёры DCGM (`direct`) или мгновенный запрос к источнику
//! истории (`prometheus`). Сбой одного экспортёра — запись в `errors`, карты остальных
//! показываются; сбой всех — ошибка первого.

use std::sync::Arc;

use serde::Serialize;
use tokio::sync::Semaphore;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::gpu::fields::{Group, all_names};
use crate::gpu::snapshot::{GpuCard, GpuHost, MAX_GPUS, cards_from, group, missing};
use crate::history::read::{Source, permit};
use crate::history::targets::{parse_selector, render};
use crate::metrics::parse::{Sample, parse, too_large};
use crate::policy::PolicyStatus;
use crate::servers::client::{METRICS_LIMIT, ServerClient, Timeouts};
use crate::storage::profile::{
    DirectGpu, GPU_NAME, GpuAuth, GpuExporter, PrometheusGpu, ServerConfig, ServerKind,
    parse_exporter_url, redact_url,
};

/// Сколько экспортёров опрашивается одновременно.
pub const CONCURRENCY: usize = 4;

/// Сбой части источника: экспортёра (`exporter` — его адрес) или всего источника.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exporter: Option<String>,
    pub error: AppError,
}

/// Снимок карт инсталляции: хосты, сбои части источника и группы счётчиков без серий.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuSnapshot {
    pub hosts: Vec<GpuHost>,
    pub errors: Vec<SourceError>,
    pub missing: Vec<Group>,
}

/// Экспортёр как адрес с настройками соединения: так его открывает общий HTTP-клиент и шаг
/// «DNS/TCP» проверки. Путь адреса — путь метрик.
pub fn exporter_server(e: &GpuExporter, d: &DirectGpu) -> AppResult<ServerConfig> {
    let url = parse_exporter_url(e)?;
    Ok(ServerConfig {
        id: crate::secrets::GPU.into(),
        name: GPU_NAME.into(),
        base_url: url.origin().ascii_serialization(),
        kind: ServerKind::Auto,
        metrics_path: url.path().to_string(),
        ca: d.ca.clone(),
        proxy: d.proxy.clone(),
        history_selector: None,
        gpus: None,
        extra: Default::default(),
    })
}

/// Отказ во входе у экспортёра называет логин и пароль экспортёров, а не ключ API сервера.
pub fn as_exporter_error(e: AppError) -> AppError {
    if e.key == "errors.server.unauthorized" {
        let status = e.params.get("status").cloned().unwrap_or_default();
        return app_err!(
            ErrorCode::AuthFailed,
            "errors.gpu.unauthorized",
            "The DCGM exporter rejected the request ({{status}}) — check the user name and password",
            status = status
        );
    }
    e
}

/// Открытый экспортёр: адрес для подписей (без имени и пароля), хост адреса и клиент.
#[derive(Debug)]
pub struct Exporter {
    pub url: String,
    pub host: String,
    client: ServerClient,
}

impl Exporter {
    pub fn new(
        e: &GpuExporter,
        d: &DirectGpu,
        timeouts: Timeouts,
        password: Option<String>,
        policy: &PolicyStatus,
    ) -> AppResult<Self> {
        let server = exporter_server(e, d)?;
        let client = ServerClient::new(&server, timeouts, None, policy)?;
        let client = match (&d.auth, password.filter(|p| !p.is_empty())) {
            (GpuAuth::Basic { username }, Some(p)) => client.with_basic_auth(username, p),
            _ => client,
        };
        let host = parse_exporter_url(e)?
            .host_str()
            .unwrap_or_default()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_string();
        Ok(Self {
            url: redact_url(&e.url),
            host,
            client,
        })
    }

    /// Серии `/metrics` экспортёра.
    pub async fn samples(&self) -> AppResult<Vec<Sample>> {
        let reply = self
            .client
            .get_ok(self.client.metrics_path(), METRICS_LIMIT)
            .await
            .map_err(as_exporter_error)?;
        if reply.truncated {
            return Err(too_large());
        }
        Ok(parse(&String::from_utf8_lossy(&reply.body))?.samples)
    }

    /// Карты экспортёра; хост карты без метки хоста — хост адреса экспортёра.
    pub async fn cards(&self) -> AppResult<Vec<GpuCard>> {
        let samples = self.samples().await?;
        let host = self.host.clone();
        Ok(cards_from(
            &samples,
            &move |_| host.clone(),
            Some(&self.url),
        ))
    }
}

/// Хост из `instance` (`host:port`, `[::1]:9400`) — без порта и скобок.
pub fn instance_host(instance: &str) -> String {
    if let Some(rest) = instance.strip_prefix('[') {
        return rest.split(']').next().unwrap_or(rest).to_string();
    }
    match instance.rsplit_once(':') {
        Some((h, p)) if !h.contains(':') && p.parse::<u16>().is_ok() => h.to_string(),
        _ => instance.to_string(),
    }
}

/// Мгновенный запрос карт к источнику истории: все серии таблицы полей под селектором карт.
pub fn instant_expr(selector: Option<&str>) -> AppResult<String> {
    let names = all_names().collect::<Vec<_>>().join("|");
    let extra = match selector {
        Some(s) => format!(",{}", render(&parse_selector(s)?)),
        None => String::new(),
    };
    Ok(format!("{{__name__=~\"{names}\"{extra}}}"))
}

/// Серии ответа источника истории в виде серий `/metrics`: имя — метка `__name__`.
pub fn from_history(samples: Vec<crate::history::client::Sample>) -> Vec<Sample> {
    samples
        .into_iter()
        .filter_map(|mut s| {
            let name = s.labels.remove("__name__")?;
            Some(Sample {
                name,
                labels: s.labels.into_iter().collect(),
                value: s.value.unwrap_or(f64::NAN),
            })
        })
        .collect()
}

/// Открытый компонент GPU инсталляции; живёт в пуле подключений до `evict` инсталляции.
#[derive(Debug)]
pub enum OpenGpu {
    Direct {
        exporters: Vec<Exporter>,
        limit: Arc<Semaphore>,
    },
    Prometheus {
        source: Arc<Source>,
        selector: Option<String>,
    },
}

impl OpenGpu {
    pub fn direct(
        d: &DirectGpu,
        timeouts: Timeouts,
        password: Option<String>,
        policy: &PolicyStatus,
    ) -> AppResult<Self> {
        let exporters = d
            .exporters
            .iter()
            .map(|e| Exporter::new(e, d, timeouts, password.clone(), policy))
            .collect::<AppResult<Vec<_>>>()?;
        Ok(Self::Direct {
            exporters,
            limit: Arc::new(Semaphore::new(CONCURRENCY)),
        })
    }

    pub fn prometheus(g: &PrometheusGpu, source: Arc<Source>) -> Self {
        Self::Prometheus {
            source,
            selector: g.selector.clone(),
        }
    }

    /// Карты и сбои экспортёров; все экспортёры (или источник) отказали — ошибка первого.
    pub async fn read(&self) -> AppResult<(Vec<GpuCard>, Vec<SourceError>)> {
        match self {
            Self::Direct { exporters, limit } => {
                let reads = exporters.iter().map(|e| {
                    let limit = limit.clone();
                    async move {
                        let _permit = limit.acquire_owned().await.expect("never closed");
                        (e, e.cards().await)
                    }
                });
                let mut cards = Vec::new();
                let mut errors = Vec::new();
                for (e, result) in futures::future::join_all(reads).await {
                    match result {
                        Ok(c) => cards.extend(c),
                        Err(error) => errors.push(SourceError {
                            exporter: Some(e.url.clone()),
                            error,
                        }),
                    }
                }
                if !errors.is_empty() && errors.len() == exporters.len() {
                    return Err(errors.swap_remove(0).error);
                }
                Ok((cards, errors))
            }
            Self::Prometheus { source, selector } => {
                let expr = instant_expr(selector.as_deref())?;
                let samples = {
                    let _permit = permit(source).await;
                    source.client.query(&expr).await?
                };
                let samples = from_history(samples);
                let cards = cards_from(
                    &samples,
                    &|s| instance_host(s.label("instance").unwrap_or_default()),
                    None,
                );
                Ok((cards, Vec::new()))
            }
        }
    }
}

/// Снимок из карт: не больше 64 карт (лишние — запись `errors.gpu.tooManyGpus`), группы
/// счётчиков без серий.
pub fn snapshot(cards: Vec<GpuCard>, mut errors: Vec<SourceError>) -> GpuSnapshot {
    let absent = missing(&cards);
    let total = cards.len();
    let (hosts, truncated) = group(cards, MAX_GPUS);
    if truncated {
        errors.push(SourceError {
            exporter: None,
            error: too_many_gpus(total),
        });
    }
    GpuSnapshot {
        hosts,
        errors,
        missing: absent,
    }
}

pub fn too_many_gpus(found: usize) -> AppError {
    app_err!(
        ErrorCode::InvalidInput,
        "errors.gpu.tooManyGpus",
        "Found {{found}} GPUs, an installation can have at most {{max}}: narrow the card selector or remove exporters",
        found = found,
        max = MAX_GPUS
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::gpu::fields::Field;
    use crate::gpu::snapshot::tests::{DEFAULT, FULL};
    use crate::history::client::HistoryClient;
    use crate::history::client::tests::source as history_source;
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::{HistoryAuth, ProxyConfig, sample_exporter, sample_gpu};

    const TIMEOUTS: Timeouts = Timeouts {
        connect_ms: 2_000,
        request_ms: 2_000,
    };

    fn direct(urls: &[String], auth: GpuAuth) -> DirectGpu {
        let GpuSource::Direct(d) = sample_gpu() else {
            unreachable!()
        };
        DirectGpu {
            exporters: urls.iter().map(|u| sample_exporter(u)).collect(),
            auth,
            proxy: ProxyConfig::None,
            ..d
        }
    }

    use crate::storage::profile::GpuSource;

    pub(crate) fn snapshot_file(name: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/src/gpu/testdata/prometheus-3.15.1/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    #[test]
    fn exporter_address_becomes_origin_and_metrics_path() {
        let d = direct(&[], GpuAuth::None);
        let s = exporter_server(&sample_exporter("https://gpu-1:9400/dcgm/metrics"), &d).unwrap();
        assert_eq!(
            (s.base_url.as_str(), s.metrics_path.as_str()),
            ("https://gpu-1:9400", "/dcgm/metrics")
        );
        assert_eq!(instance_host("10.0.0.5:9400"), "10.0.0.5");
        assert_eq!(instance_host("[::1]:9400"), "::1");
        assert_eq!(instance_host("gpu-node-1"), "gpu-node-1");
    }

    #[test]
    fn instant_expression_lists_every_field_series_under_the_selector() {
        let all = instant_expr(None).unwrap();
        assert!(
            all.starts_with(r#"{__name__=~"DCGM_FI_DEV_GPU_UTIL|DCGM_FI_PROF_SM_ACTIVE|"#),
            "{all}"
        );
        assert!(
            all.ends_with(r#"|DCGM_FI_DEV_NVLINK_REPLAY_TOTAL"}"#),
            "{all}"
        );
        let narrowed = instant_expr(Some(r#"{hostname=~"gpu-1|gpu-2", job="dcgm"}"#)).unwrap();
        assert!(
            narrowed.ends_with(r#"_TOTAL",hostname=~"gpu-1|gpu-2",job="dcgm"}"#),
            "{narrowed}"
        );
        assert_eq!(
            instant_expr(Some("job=dcgm")).unwrap_err().key,
            "errors.history.selector"
        );
    }

    #[tokio::test]
    async fn direct_reads_every_exporter_and_survives_a_failed_one() {
        let a = TestServer::start(vec![("/metrics", Reply::text(FULL))]).await;
        let b = TestServer::start(vec![("/dcgm/metrics", Reply::text(DEFAULT))]).await;
        let down = TestServer::start(vec![("/metrics", Reply::status(503))]).await;
        let urls = [
            format!("{}/metrics", a.url()),
            format!("{}/dcgm/metrics", b.url()),
            format!("{}/metrics", down.url()),
        ];
        let open = OpenGpu::direct(
            &direct(&urls, GpuAuth::None),
            TIMEOUTS,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        let (cards, errors) = open.read().await.unwrap();
        assert_eq!(cards.len(), 10);
        assert_eq!(
            cards
                .iter()
                .filter(|c| c.exporter.as_deref() == Some(urls[1].as_str()))
                .count(),
            2
        );
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].exporter.as_deref(), Some(urls[2].as_str()));
        assert_eq!(errors[0].error.key, "errors.server.serverError");
        let snap = snapshot(cards, errors);
        assert_eq!(
            snap.hosts
                .iter()
                .map(|h| h.host.as_str())
                .collect::<Vec<_>>(),
            ["gpu-node-1", "localhost"]
        );
        assert!(
            snap.missing.is_empty(),
            "полный набор есть у одного экспортёра"
        );

        let only_down = OpenGpu::direct(
            &direct(&urls[2..], GpuAuth::None),
            TIMEOUTS,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        assert_eq!(
            only_down.read().await.unwrap_err().key,
            "errors.server.serverError",
            "сбой всех — ошибка"
        );
    }

    #[tokio::test]
    async fn basic_login_goes_to_exporters_and_a_refusal_names_the_password() {
        let s = TestServer::start(vec![("/metrics", Reply::text(DEFAULT))]).await;
        let urls = [format!("{}/metrics", s.url())];
        let auth = GpuAuth::Basic {
            username: "glafira".into(),
        };
        let open = OpenGpu::direct(
            &direct(&urls, auth.clone()),
            TIMEOUTS,
            Some("dcgm-pa55".into()),
            &PolicyStatus::default(),
        )
        .unwrap();
        open.read().await.unwrap();
        let seen = s.seen.lock().unwrap().last().cloned().unwrap();
        assert_eq!(
            seen.authorization.as_deref(),
            Some("Basic Z2xhZmlyYTpkY2dtLXBhNTU=")
        );
        assert!(!format!("{open:?}").contains("dcgm-pa55"));

        let refused = TestServer::start(vec![("/metrics", Reply::status(401))]).await;
        let urls = [format!("{}/metrics", refused.url())];
        let open = OpenGpu::direct(
            &direct(&urls, auth),
            TIMEOUTS,
            Some("wrong".into()),
            &PolicyStatus::default(),
        )
        .unwrap();
        let e = open.read().await.unwrap_err();
        assert_eq!(
            (e.key.as_str(), e.params["status"].as_str()),
            ("errors.gpu.unauthorized", "401")
        );
    }

    #[tokio::test]
    async fn too_large_metrics_are_an_error() {
        let big = "x".repeat(METRICS_LIMIT + 10);
        let s = TestServer::start(vec![("/metrics", Reply::text(&big))]).await;
        let urls = [format!("{}/metrics", s.url())];
        let open = OpenGpu::direct(
            &direct(&urls, GpuAuth::None),
            TIMEOUTS,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        assert_eq!(
            open.read().await.unwrap_err().key,
            "errors.metrics.tooLarge"
        );
    }

    #[tokio::test]
    async fn prometheus_reads_cards_from_a_real_instant_answer() {
        let p = TestServer::start(vec![(
            "/api/v1/query",
            Reply::json(&snapshot_file("instant.json")),
        )])
        .await;
        let client = HistoryClient::new(
            &history_source(&p.url(), HistoryAuth::None),
            TIMEOUTS,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        let source = Arc::new(Source::new(
            client,
            crate::history::client::SourceKind::Prometheus,
            std::time::Duration::from_secs(5),
        ));
        let open = OpenGpu::prometheus(
            &PrometheusGpu {
                selector: Some(r#"{job="dcgm"}"#.into()),
                extra: Default::default(),
            },
            source,
        );
        let (cards, errors) = open.read().await.unwrap();
        assert!(errors.is_empty());
        let snap = snapshot(cards, errors);
        assert_eq!(
            snap.hosts
                .iter()
                .map(|h| (h.host.as_str(), h.cards.len()))
                .collect::<Vec<_>>(),
            [("gpu-node-1", 8), ("localhost", 2)]
        );
        let card = &snap.hosts[0].cards[0];
        assert_eq!(card.model.as_deref(), Some("NVIDIA H100 80GB HBM3"));
        assert_eq!(card.values[&Field::MemTotal], 81559.0);
        assert!(card.values.contains_key(&Field::SmActive));
        assert_eq!(card.exporter, None);
        let path = p.paths().pop().unwrap();
        assert!(
            path.contains("job%3D%22dcgm%22") || path.contains("job=%22dcgm%22"),
            "{path}"
        );
    }

    #[test]
    fn more_than_64_cards_are_cut_with_an_error() {
        let text: String = (0..9)
            .map(|n| FULL.replace("gpu-node-1", &format!("gpu-{n}")))
            .collect();
        let cards = crate::gpu::snapshot::tests::cards(&text, "x");
        assert_eq!(cards.len(), 72);
        let snap = snapshot(cards, Vec::new());
        assert_eq!(
            snap.hosts.iter().map(|h| h.cards.len()).sum::<usize>(),
            MAX_GPUS
        );
        assert_eq!(snap.errors[0].error.key, "errors.gpu.tooManyGpus");
        assert_eq!(snap.errors[0].error.params["found"], "72");
        let v = serde_json::to_value(&snap.errors[0]).unwrap();
        assert!(v.get("exporter").is_none(), "{v}");
    }
}
