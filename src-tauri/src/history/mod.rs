//! История метрик из Prometheus или VictoriaMetrics (спека I1c): клиент источника, цели и
//! селекторы серий, шаблоны PromQL и окна.

pub mod client;
pub mod promql;
pub mod series;
pub mod targets;
pub mod window;
