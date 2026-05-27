use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use shared::{AdminIdentityView, UserIdentityView};
use url::Url;
use uuid::Uuid;

use crate::model::{AppState, MicrosoftAuthConfig, MicrosoftAuthKind, MicrosoftAuthRequest};

const MICROSOFT_SCOPE: &str = "openid email profile User.Read";
const MICROSOFT_USERINFO_URL: &str = "https://graph.microsoft.com/oidc/userinfo";

pub fn microsoft_auth_config() -> Option<Arc<MicrosoftAuthConfig>> {
    let client_id = env::var("MICROSOFT_CLIENT_ID").ok()?;
    let client_secret = env::var("MICROSOFT_CLIENT_SECRET").ok()?;
    let tenant_id = env::var("MICROSOFT_TENANT_ID").unwrap_or_else(|_| "common".to_string());
    let server_base_url =
        env::var("SERVER_PUBLIC_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    let redirect_uri = env::var("MICROSOFT_REDIRECT_URI")
        .unwrap_or_else(|_| format!("{server_base_url}/auth/microsoft/callback"));
    let frontend_base_url = frontend_base_url();

    Some(Arc::new(MicrosoftAuthConfig {
        client_id,
        client_secret,
        tenant_id,
        redirect_uri,
        frontend_base_url,
        http_client: reqwest::Client::new(),
    }))
}

pub async fn microsoft_start_handler(
    State(state): State<AppState>,
    Query(query): Query<MicrosoftStartQuery>,
) -> Response {
    let frontend_base_url = frontend_base_url();
    match microsoft_start_url(&state, query).await {
        Ok(url) => Redirect::temporary(url.as_str()).into_response(),
        Err(message) => redirect_to_auth_error(&frontend_base_url, message),
    }
}

pub async fn microsoft_callback_handler(
    State(state): State<AppState>,
    Query(query): Query<MicrosoftCallbackQuery>,
) -> Response {
    let frontend_base_url = state
        .microsoft_auth
        .as_ref()
        .map(|config| config.frontend_base_url.clone())
        .unwrap_or_else(frontend_base_url);
    match microsoft_callback(&state, query).await {
        Ok(url) => Redirect::temporary(url.as_str()).into_response(),
        Err(message) => redirect_to_auth_error(&frontend_base_url, message),
    }
}

#[derive(Deserialize)]
pub struct MicrosoftStartQuery {
    kind: Option<MicrosoftAuthKind>,
    return_to: Option<String>,
}

#[derive(Deserialize)]
pub struct MicrosoftCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Deserialize)]
struct MicrosoftTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct MicrosoftUserInfo {
    name: Option<String>,
    email: Option<String>,
    preferred_username: Option<String>,
}

struct MicrosoftProfile {
    name: Option<String>,
    email: String,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum AuthCompleteSession {
    Admin { session: AdminIdentityView },
    User { session: UserIdentityView },
}

async fn microsoft_start_url(state: &AppState, query: MicrosoftStartQuery) -> Result<Url, String> {
    let config = state
        .microsoft_auth
        .as_ref()
        .ok_or_else(|| "Microsoft SSO is not configured".to_string())?;
    let kind = query.kind.unwrap_or(MicrosoftAuthKind::User);
    {
        let store = state.store.read().await;
        match kind {
            MicrosoftAuthKind::Admin if !store.site_settings.admin_microsoft_sign_in_enabled => {
                return Err("admin Microsoft sign-in is disabled".to_string());
            }
            MicrosoftAuthKind::User if !store.site_settings.user_microsoft_sign_in_enabled => {
                return Err("user Microsoft sign-in is disabled".to_string());
            }
            _ => {}
        }
    }
    let return_to = normalize_return_to(query.return_to.unwrap_or_else(|| "/".to_string()));
    let csrf_state = Uuid::new_v4().to_string();

    state
        .microsoft_auth_requests
        .write()
        .await
        .insert(csrf_state.clone(), MicrosoftAuthRequest { kind, return_to });

    let mut url = Url::parse(&format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/authorize",
        config.tenant_id
    ))
    .map_err(|error| format!("failed to build Microsoft authorize URL: {error}"))?;
    url.query_pairs_mut()
        .append_pair("client_id", &config.client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &config.redirect_uri)
        .append_pair("response_mode", "query")
        .append_pair("scope", MICROSOFT_SCOPE)
        .append_pair("state", &csrf_state);

    Ok(url)
}

async fn microsoft_callback(
    state: &AppState,
    query: MicrosoftCallbackQuery,
) -> Result<Url, String> {
    if let Some(error) = query.error {
        return Err(query
            .error_description
            .unwrap_or_else(|| format!("Microsoft sign-in failed: {error}")));
    }

    let csrf_state = query
        .state
        .ok_or_else(|| "Microsoft sign-in response was missing state".to_string())?;
    let request = state
        .microsoft_auth_requests
        .write()
        .await
        .remove(&csrf_state)
        .ok_or_else(|| "Microsoft sign-in state expired or was invalid".to_string())?;
    let code = query
        .code
        .ok_or_else(|| "Microsoft sign-in response was missing code".to_string())?;

    let config = state
        .microsoft_auth
        .as_ref()
        .ok_or_else(|| "Microsoft SSO is not configured".to_string())?;
    let profile = exchange_code_for_profile(config, code).await?;

    let complete_session = {
        let mut store = state.store.write().await;
        let session = match request.kind {
            MicrosoftAuthKind::Admin => AuthCompleteSession::Admin {
                session: store.login_admin_with_email(profile.email)?,
            },
            MicrosoftAuthKind::User => AuthCompleteSession::User {
                session: store.login_or_create_user_with_microsoft(profile.email, profile.name)?,
            },
        };
        store
            .save_to_disk(&state.data_path)
            .map_err(|error| format!("failed to save session: {error}"))?;
        session
    };

    auth_complete_url(config, &request.return_to, &complete_session)
}

async fn exchange_code_for_profile(
    config: &MicrosoftAuthConfig,
    code: String,
) -> Result<MicrosoftProfile, String> {
    let token_endpoint = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
        config.tenant_id
    );
    let token_response = config
        .http_client
        .post(token_endpoint)
        .form(&HashMap::from([
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("code", code.as_str()),
            ("redirect_uri", config.redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ]))
        .send()
        .await
        .map_err(|error| format!("failed to exchange Microsoft code: {error}"))?;

    if !token_response.status().is_success() {
        return Err(format!(
            "Microsoft token exchange failed with status {}",
            token_response.status()
        ));
    }

    let token = token_response
        .json::<MicrosoftTokenResponse>()
        .await
        .map_err(|error| format!("failed to read Microsoft token response: {error}"))?;
    let userinfo_response = config
        .http_client
        .get(MICROSOFT_USERINFO_URL)
        .bearer_auth(token.access_token)
        .send()
        .await
        .map_err(|error| format!("failed to load Microsoft user profile: {error}"))?;

    if !userinfo_response.status().is_success() {
        return Err(format!(
            "Microsoft user profile request failed with status {}",
            userinfo_response.status()
        ));
    }

    let userinfo = userinfo_response
        .json::<MicrosoftUserInfo>()
        .await
        .map_err(|error| format!("failed to read Microsoft user profile: {error}"))?;
    let email = userinfo
        .email
        .or(userinfo.preferred_username)
        .ok_or_else(|| "Microsoft profile did not include an email address".to_string())?;

    Ok(MicrosoftProfile {
        name: userinfo.name,
        email,
    })
}

fn auth_complete_url(
    config: &MicrosoftAuthConfig,
    return_to: &str,
    session: &AuthCompleteSession,
) -> Result<Url, String> {
    let mut url = Url::parse(&config.frontend_base_url)
        .map_err(|error| format!("invalid FRONTEND_BASE_URL: {error}"))?;
    url.set_path("/auth/microsoft/complete");
    url.set_query(None);
    let session_payload = serde_json::to_vec(session)
        .map_err(|error| format!("failed to serialize Microsoft session: {error}"))?;
    url.query_pairs_mut()
        .append_pair("session", &URL_SAFE_NO_PAD.encode(session_payload))
        .append_pair("return_to", &URL_SAFE_NO_PAD.encode(return_to.as_bytes()));
    Ok(url)
}

fn auth_error_url(frontend_base_url: &str, message: &str) -> Result<Url, String> {
    let mut url = Url::parse(frontend_base_url)
        .map_err(|error| format!("invalid FRONTEND_BASE_URL: {error}"))?;
    url.set_path("/auth/microsoft/complete");
    url.set_query(None);
    url.query_pairs_mut()
        .append_pair("error", &URL_SAFE_NO_PAD.encode(message.as_bytes()));
    Ok(url)
}

fn redirect_to_auth_error(frontend_base_url: &str, message: String) -> Response {
    match auth_error_url(frontend_base_url, &message) {
        Ok(url) => Redirect::temporary(url.as_str()).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            format!("{message}; also failed to build frontend error redirect: {error}"),
        )
            .into_response(),
    }
}

fn frontend_base_url() -> String {
    env::var("FRONTEND_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string())
}

fn normalize_return_to(value: String) -> String {
    if value.starts_with('/') && !value.starts_with("//") {
        value
    } else {
        "/".to_string()
    }
}
