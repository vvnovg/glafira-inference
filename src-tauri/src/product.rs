//! Имя продукта и идентификаторы — в одном месте. Переименование меняет только этот файл
//! и `tauri.conf.json`; тест ниже следит, чтобы они совпадали.

pub const PRODUCT_NAME: &str = "Glafira for LLM Inference";
pub const IDENTIFIER: &str = "io.github.vvnovg.glafira.inference";
pub const KEYCHAIN_SERVICE: &str = "glafira-inference";
pub const POLICY_VENDOR: &str = "Glafira";
pub const POLICY_PRODUCT: &str = "Inference";
/// Имя, под которым Glafira видна серверам инференса: `User-Agent` запросов.
pub const CLIENT_NAME: &str = "glafira-inference";

#[cfg(test)]
mod tests {
    #[test]
    fn matches_tauri_conf() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(conf["identifier"], super::IDENTIFIER);
        assert_eq!(conf["productName"], super::PRODUCT_NAME);
        assert_eq!(conf["app"]["windows"][0]["title"], super::PRODUCT_NAME);
        assert_eq!(conf["mainBinaryName"], super::CLIENT_NAME);
    }
}
