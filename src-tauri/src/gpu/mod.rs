//! GPU инсталляции из экспортёров DCGM (спека I1d): поля карт, снимок карт по хостам, автоподбор
//! привязок, источник карт и приросты счётчиков, окна истории.

pub mod bind;
pub mod fields;
pub mod live;
pub mod promql;
pub mod series;
pub mod snapshot;
pub mod source;
