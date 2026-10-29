//! Серверы инференса (спец. §4.3): HTTP-клиент из профиля, ошибки сервера, определение типа,
//! версии и моделей.

pub mod client;
pub mod detect;
pub mod errors;
pub mod probe;
#[cfg(test)]
pub mod testing;
