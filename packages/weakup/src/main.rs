use api::{ApnsConfig, FcmConfig, ServerConfig, WakeupPlatform, WakeupRequest, WakeupResponse};
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn config_path_from_args() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" | "-c" => {
                if let Some(path) = args.next() {
                    return Some(PathBuf::from(path));
                }
            }
            _ => {
                if !arg.starts_with('-') {
                    return Some(PathBuf::from(arg));
                }
            }
        }
    }
    None
}

#[derive(Clone)]
struct AppState {
    client: Client,
    config: ServerConfig,
    fcm: Option<FcmServiceAccount>,
    apns: Option<ApnsAuth>,
}

#[derive(Clone, Debug, Deserialize)]
struct FcmServiceAccount {
    project_id: String,
    client_email: String,
    private_key: String,
    #[serde(default)]
    token_uri: Option<String>,
}

#[derive(Clone, Debug)]
struct ApnsAuth {
    key_id: String,
    team_id: String,
    bundle_id: String,
    private_key: String,
    use_sandbox: bool,
}

#[derive(serde::Serialize)]
struct GoogleJwtClaims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    iat: usize,
    exp: usize,
}

#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

#[derive(serde::Serialize)]
struct ApnsJwtClaims<'a> {
    iss: &'a str,
    iat: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = config_path_from_args().unwrap_or_else(ServerConfig::config_path_from_env);
    let config = ServerConfig::load_or_create(&config_path)?;
    let fcm = load_fcm_service_account(config.fcm.as_ref()).await?;
    let apns = load_apns_auth(config.apns.as_ref()).await?;

    let state = AppState {
        client: Client::new(),
        config,
        fcm,
        apns,
    };

    let addr = state.config.socket_addr();
    let app = Router::new().route("/wake", post(wake_handler)).with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("WeakUp server listening on {addr}");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn wake_handler(
    State(state): State<AppState>,
    Json(request): Json<WakeupRequest>,
) -> Result<Json<WakeupResponse>, (StatusCode, String)> {
    let result = match request.platform {
        WakeupPlatform::Fcm => send_fcm(&state, &request).await,
        WakeupPlatform::Apns => send_apns(&state, &request).await,
    };

    match result {
        Ok(message_id) => Ok(Json(WakeupResponse {
            success: true,
            message_id: Some(message_id),
            error: None,
        })),
        Err(error) => Ok(Json(WakeupResponse {
            success: false,
            message_id: None,
            error: Some(error),
        })),
    }
}

async fn load_fcm_service_account(
    config: Option<&FcmConfig>,
) -> Result<Option<FcmServiceAccount>, String> {
    let Some(config) = config else {
        return Ok(None);
    };
    let content = tokio::fs::read_to_string(&config.service_account_path)
        .await
        .map_err(|err| format!("FCM service account read error: {err}"))?;
    let account: FcmServiceAccount = serde_json::from_str(&content)
        .map_err(|err| format!("FCM service account parse error: {err}"))?;
    Ok(Some(account))
}

async fn load_apns_auth(config: Option<&ApnsConfig>) -> Result<Option<ApnsAuth>, String> {
    let Some(config) = config else {
        return Ok(None);
    };
    let private_key = tokio::fs::read_to_string(&config.private_key_path)
        .await
        .map_err(|err| format!("APNs key read error: {err}"))?;
    Ok(Some(ApnsAuth {
        key_id: config.key_id.clone(),
        team_id: config.team_id.clone(),
        bundle_id: config.bundle_id.clone(),
        private_key,
        use_sandbox: config.use_sandbox,
    }))
}

async fn send_fcm(state: &AppState, request: &WakeupRequest) -> Result<String, String> {
    let account = state
        .fcm
        .as_ref()
        .ok_or_else(|| "FCM not configured".to_string())?;
    let token_uri = account
        .token_uri
        .clone()
        .unwrap_or_else(|| "https://oauth2.googleapis.com/token".to_string());
    let now = unix_timestamp();
    let claims = GoogleJwtClaims {
        iss: &account.client_email,
        scope: "https://www.googleapis.com/auth/firebase.messaging",
        aud: &token_uri,
        iat: now,
        exp: now + 3600,
    };
    let mut header = Header::new(Algorithm::RS256);
    header.typ = Some("JWT".to_string());
    let jwt = jsonwebtoken::encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(account.private_key.as_bytes())
            .map_err(|err| format!("FCM key parse error: {err}"))?,
    )
    .map_err(|err| format!("FCM JWT error: {err}"))?;

    let token_response = state
        .client
        .post(&token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", jwt.as_str()),
        ])
        .send()
        .await
        .map_err(|err| format!("FCM token request error: {err}"))?;

    let token_response = token_response
        .error_for_status()
        .map_err(|err| format!("FCM token response error: {err}"))?;
    let token: GoogleTokenResponse = token_response
        .json()
        .await
        .map_err(|err| format!("FCM token decode error: {err}"))?;

    let mut message = serde_json::json!({
        "message": {
            "token": request.device_token,
        }
    });

    if request.title.is_some() || request.body.is_some() {
        message["message"]["notification"] = serde_json::json!({
            "title": request.title.clone().unwrap_or_default(),
            "body": request.body.clone().unwrap_or_default(),
        });
    }

    if let Some(data) = request.data.as_ref().and_then(as_string_map) {
        message["message"]["data"] = serde_json::to_value(data)
            .map_err(|err| format!("FCM data serialize error: {err}"))?;
    }

    let url = format!(
        "https://fcm.googleapis.com/v1/projects/{}/messages:send",
        account.project_id
    );
    let response = state
        .client
        .post(&url)
        .bearer_auth(&token.access_token)
        .json(&message)
        .send()
        .await
        .map_err(|err| format!("FCM send error: {err}"))?;

    let response = response
        .error_for_status()
        .map_err(|err| format!("FCM send response error: {err}"))?;
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|err| format!("FCM response decode error: {err}"))?;
    let message_id = value
        .get("name")
        .and_then(|val| val.as_str())
        .unwrap_or("fcm_unknown")
        .to_string();
    Ok(message_id)
}

async fn send_apns(state: &AppState, request: &WakeupRequest) -> Result<String, String> {
    let auth = state
        .apns
        .as_ref()
        .ok_or_else(|| "APNs not configured".to_string())?;
    let now = unix_timestamp();
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(auth.key_id.clone());
    let claims = ApnsJwtClaims {
        iss: &auth.team_id,
        iat: now,
    };
    let jwt = jsonwebtoken::encode(
        &header,
        &claims,
        &EncodingKey::from_ec_pem(auth.private_key.as_bytes())
            .map_err(|err| format!("APNs key parse error: {err}"))?,
    )
    .map_err(|err| format!("APNs JWT error: {err}"))?;

    let payload = build_apns_payload(request)?;
    let push_type = if request.title.is_some() || request.body.is_some() {
        "alert"
    } else {
        "background"
    };
    let priority = if push_type == "alert" { "10" } else { "5" };

    let url = if auth.use_sandbox {
        format!("https://api.sandbox.push.apple.com/3/device/{}", request.device_token)
    } else {
        format!("https://api.push.apple.com/3/device/{}", request.device_token)
    };

    let response = state
        .client
        .post(&url)
        .header("authorization", format!("bearer {jwt}"))
        .header("apns-topic", &auth.bundle_id)
        .header("apns-push-type", push_type)
        .header("apns-priority", priority)
        .json(&payload)
        .send()
        .await
        .map_err(|err| format!("APNs send error: {err}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("APNs error {status}: {body}"));
    }

    let message_id = response
        .headers()
        .get("apns-id")
        .and_then(|val| val.to_str().ok())
        .unwrap_or("apns_unknown")
        .to_string();
    Ok(message_id)
}

fn build_apns_payload(request: &WakeupRequest) -> Result<serde_json::Value, String> {
    let mut aps = serde_json::json!({});
    if request.title.is_some() || request.body.is_some() {
        aps["alert"] = serde_json::json!({
            "title": request.title.clone().unwrap_or_default(),
            "body": request.body.clone().unwrap_or_default(),
        });
    } else {
        aps["content-available"] = serde_json::json!(1);
    }

    let mut payload = serde_json::json!({ "aps": aps });
    if let Some(data) = request.data.as_ref().and_then(as_string_map) {
        payload["data"] = serde_json::to_value(data)
            .map_err(|err| format!("APNs data serialize error: {err}"))?;
    }
    Ok(payload)
}

fn as_string_map(data: &serde_json::Value) -> Option<BTreeMap<String, String>> {
    match data {
        serde_json::Value::Object(map) => {
            let mut result = BTreeMap::new();
            for (key, value) in map {
                let value = match value {
                    serde_json::Value::String(val) => val.clone(),
                    _ => value.to_string(),
                };
                result.insert(key.clone(), value);
            }
            Some(result)
        }
        _ => None,
    }
}

fn unix_timestamp() -> usize {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as usize
}
