// codex-wire-protocol/src/auth.rs

#[derive(Debug, Clone)]
pub enum AuthToken {
    Bearer(String),
    ApiKey {
        key: String,
        header_name: String,
    },
    AwsSigV4 {
        access_key: String,
        secret_key: String,
        session_token: Option<String>,
        region: String,
        service: String,
    },
    GcpAuth {
        token: String,
    },
    None,
}
