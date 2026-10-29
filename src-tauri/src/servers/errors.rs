//! Ошибки сервера инференса и сети → `AppError` (спец. §16). Классификация сетевых ошибок — по
//! тексту цепочки `source()`, как в Glafira for Kubernetes: так её можно применить к ошибкам любого
//! HTTP-клиента.

use crate::error::{AppError, ErrorCode};

/// Текст ошибки вместе со всеми причинами: «a: b: c».
pub fn chain_text(e: &dyn std::error::Error) -> String {
    let mut out = e.to_string();
    let mut cur = e.source();
    while let Some(s) = cur {
        let t = s.to_string();
        if !out.contains(&t) {
            out.push_str(": ");
            out.push_str(&t);
        }
        cur = s.source();
    }
    out
}

/// Сетевая ошибка по тексту цепочки причин (hyper, rustls, DNS).
pub fn from_transport_text(text: &str) -> AppError {
    let t = text.to_lowercase();
    let e = if t.contains("invalid peer certificate") || t.contains("certificate") {
        if t.contains("notvalidforname") || t.contains("not valid for name") {
            app_err!(
                ErrorCode::TlsError,
                "errors.server.hostnameMismatch",
                "The server certificate does not match its address — check the address"
            )
        } else if t.contains("expired") || t.contains("notvalidyet") {
            app_err!(
                ErrorCode::TlsError,
                "errors.server.certExpired",
                "The server certificate has expired or is not valid yet"
            )
        } else if t.contains("unknownissuer")
            || t.contains("not trusted")
            || t.contains("untrusted")
        {
            app_err!(
                ErrorCode::TlsError,
                "errors.server.caUntrusted",
                "The server certificate is not signed by a trusted CA — specify the CA of the server"
            )
        } else {
            app_err!(
                ErrorCode::TlsError,
                "errors.server.tlsError",
                "TLS connection error"
            )
        }
    } else if t.contains("tls")
        || t.contains("handshake")
        // Профиль с `https://`, а сервер отвечает по обычному HTTP: на ClientHello приходит
        // «400 Bad Request» текстом, и rustls сообщает «received corrupt message of type
        // InvalidContentType» — без слов «tls» и «certificate». Это ошибка TLS, а не отказ
        // в соединении.
        || t.contains("corrupt message")
        || t.contains("invalidcontenttype")
    {
        app_err!(
            ErrorCode::TlsError,
            "errors.server.tlsError",
            "TLS connection error"
        )
    } else if t.contains("dns error") || t.contains("failed to lookup address") {
        app_err!(
            ErrorCode::ConnectionFailed,
            "errors.server.dns",
            "Cannot resolve the server host name"
        )
    } else if t.contains("timed out") || t.contains("elapsed") || t.contains("timeout") {
        timeout()
    } else if t.contains("connect") || t.contains("refused") || t.contains("reset") {
        app_err!(
            ErrorCode::ConnectionFailed,
            "errors.server.connect",
            "Cannot connect to the server"
        )
    } else {
        app_err!(
            ErrorCode::Internal,
            "errors.server.transport",
            "Network error: {{message}}",
            message = text
        )
    };
    e.with_details(text)
}

/// Ошибка, если операция не уложилась в таймаут запроса.
pub fn timeout() -> AppError {
    app_err!(
        ErrorCode::Timeout,
        "errors.server.timeout",
        "The server did not respond in time"
    )
}

/// Текст цепочки ошибки reqwest без адреса запроса. Reqwest пишет адрес в сообщение
/// («error sending request for url (…)»), а слова из хоста и пути («tls», «connect») сбили бы
/// классификацию по подстрокам; к тому же адрес незачем показывать в `details`.
fn reqwest_text(e: &reqwest::Error) -> String {
    let text = chain_text(e);
    match e.url() {
        Some(url) => text
            .replace(&format!(" for url ({url})"), "")
            .replace(url.as_str(), ""),
        None => text,
    }
}

/// Ошибка HTTP-клиента: таймаут или сетевая ошибка по тексту цепочки (без адреса запроса).
pub fn from_reqwest(e: &reqwest::Error) -> AppError {
    let text = reqwest_text(e);
    if e.is_timeout() {
        timeout().with_details(text)
    } else {
        from_transport_text(&text)
    }
}

/// Ответ сервера с кодом не 2xx на запрос `path`. Тело ответа в текст не попадает: сервер мог
/// вернуть в нём что угодно, а ошибка уходит в интерфейс.
pub fn from_status(status: u16, path: &str) -> AppError {
    match status {
        401 | 403 => app_err!(
            ErrorCode::AuthFailed,
            "errors.server.unauthorized",
            "The server rejected the request ({{status}}) — check the API key",
            status = status
        ),
        404 => app_err!(
            ErrorCode::NotFound,
            "errors.server.notFound",
            "The server has no {{path}} (404)",
            path = path
        ),
        300..=399 => app_err!(
            ErrorCode::ConnectionFailed,
            "errors.server.redirect",
            "The server redirects {{path}} to another address ({{status}}) — use the final address of the server",
            path = path,
            status = status
        ),
        500..=599 => app_err!(
            ErrorCode::ConnectionFailed,
            "errors.server.serverError",
            "The server failed to answer {{path}} ({{status}})",
            path = path,
            status = status
        ),
        _ => app_err!(
            ErrorCode::Internal,
            "errors.server.unexpectedStatus",
            "Unexpected answer to {{path}}: {{status}}",
            path = path,
            status = status
        ),
    }
}

/// С такой ошибкой клиент сервера дальше не работает: ключ API или TLS. Следующее чтение должно
/// подключиться заново.
pub fn breaks_client(code: ErrorCode) -> bool {
    matches!(code, ErrorCode::AuthFailed | ErrorCode::TlsError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_texts() {
        for (text, code, key) in [
            (
                "error sending request: client error (Connect): invalid peer certificate: NotValidForName",
                ErrorCode::TlsError,
                "errors.server.hostnameMismatch",
            ),
            (
                "client error (Connect): invalid peer certificate: UnknownIssuer",
                ErrorCode::TlsError,
                "errors.server.caUntrusted",
            ),
            (
                "invalid peer certificate: Other(OtherError(\"“*.badssl.com” certificate is not trusted: -67843\"))",
                ErrorCode::TlsError,
                "errors.server.caUntrusted",
            ),
            (
                "invalid peer certificate: Other(OtherError(\"“*.badssl.com” certificate is expired: -67818\"))",
                ErrorCode::TlsError,
                "errors.server.certExpired",
            ),
            (
                "invalid peer certificate: BadSignature",
                ErrorCode::TlsError,
                "errors.server.tlsError",
            ),
            (
                "client error (Connect): received corrupt message of type InvalidContentType",
                ErrorCode::TlsError,
                "errors.server.tlsError",
            ),
            (
                "client error (Connect): dns error: failed to lookup address information",
                ErrorCode::ConnectionFailed,
                "errors.server.dns",
            ),
            (
                "client error (Connect): tcp connect error: Connection refused (os error 61)",
                ErrorCode::ConnectionFailed,
                "errors.server.connect",
            ),
            (
                "deadline has elapsed",
                ErrorCode::Timeout,
                "errors.server.timeout",
            ),
            (
                "something else",
                ErrorCode::Internal,
                "errors.server.transport",
            ),
        ] {
            let e = from_transport_text(text);
            assert_eq!((e.code, e.key.as_str()), (code, key), "{text}");
            assert_eq!(e.details.as_deref(), Some(text));
        }
        assert!(from_transport_text("tcp connect error").retryable);
    }

    #[test]
    fn statuses() {
        for (status, code, key) in [
            (401, ErrorCode::AuthFailed, "errors.server.unauthorized"),
            (403, ErrorCode::AuthFailed, "errors.server.unauthorized"),
            (404, ErrorCode::NotFound, "errors.server.notFound"),
            (302, ErrorCode::ConnectionFailed, "errors.server.redirect"),
            (
                503,
                ErrorCode::ConnectionFailed,
                "errors.server.serverError",
            ),
            (418, ErrorCode::Internal, "errors.server.unexpectedStatus"),
        ] {
            let e = from_status(status, "/v1/models");
            assert_eq!((e.code, e.key.as_str()), (code, key), "{status}");
        }
        assert_eq!(from_status(404, "/version").params["path"], "/version");
        assert!(breaks_client(ErrorCode::AuthFailed) && breaks_client(ErrorCode::TlsError));
        assert!(!breaks_client(ErrorCode::Timeout));
    }
}
