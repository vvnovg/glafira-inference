//! Серверы инференса (спец. §4.3): HTTP-клиент из профиля, ошибки сервера, определение типа,
//! версии и моделей, подключённые инсталляции, пошаговая проверка подключения.

pub mod adapters;
pub mod client;
pub mod connectivity;
pub mod detect;
pub mod errors;
pub mod pool;
pub mod probe;
#[cfg(test)]
pub mod testing;
