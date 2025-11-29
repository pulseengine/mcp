//! OAuth 2.1 Authorization Endpoint with PKCE
//!
//! Implements authorization code flow with mandatory PKCE (S256)

use crate::oauth::models::{AuthorizeRequest, OAuthError};
use crate::oauth::pkce::{validate_code_challenge, validate_code_verifier};
use axum::{
    extract::Query,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;

/// GET /oauth/authorize - Display authorization consent form
///
/// OAuth 2.1 authorization request with PKCE.
///
/// # Query Parameters
/// - `response_type`: Must be "code"
/// - `client_id`: Registered client identifier
/// - `redirect_uri`: Must match one of client's registered URIs
/// - `state`: Recommended opaque value for CSRF protection
/// - `code_challenge`: PKCE S256 code challenge
/// - `code_challenge_method`: Must be "S256"
/// - `resource`: (Optional) RFC 8707 resource indicator
/// - `scope`: (Optional) Space-separated scopes
///
/// # Example
/// ```
/// GET /oauth/authorize?
///   response_type=code&
///   client_id=abc123&
///   redirect_uri=https://example.com/callback&
///   state=xyz&
///   code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM&
///   code_challenge_method=S256&
///   scope=mcp:read mcp:write
/// ```
pub async fn authorize_get(
    Query(params): Query<AuthorizeRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<OAuthError>)> {
    // Validate request parameters
    validate_authorize_request(&params)?;

    // TODO: Verify client_id exists in database
    // TODO: Verify redirect_uri matches registered URIs for this client

    // Render consent form
    let html = render_consent_form(&params);
    Ok(Html(html))
}

/// POST /oauth/authorize - User consent submission
///
/// Handles user's authorization decision (approve/deny).
///
/// # Form Parameters
/// - `client_id`: Client identifier
/// - `redirect_uri`: Redirect URI from original request
/// - `state`: State from original request
/// - `code_challenge`: PKCE code challenge
/// - `resource`: (Optional) Resource indicator
/// - `scope`: (Optional) Requested scopes
/// - `approved`: "true" if user approved, "false" if denied
#[derive(Debug, Deserialize)]
pub struct AuthorizeForm {
    pub client_id: String,
    pub redirect_uri: String,
    pub state: Option<String>,
    pub code_challenge: String,
    pub resource: Option<String>,
    pub scope: Option<String>,
    pub approved: String,
}

pub async fn authorize_post(
    Form(form): Form<AuthorizeForm>,
) -> Result<impl IntoResponse, (StatusCode, Json<OAuthError>)> {
    // Check if user denied authorization
    if form.approved != "true" {
        let error_redirect = format!(
            "{}?error=access_denied&error_description=User denied authorization{}",
            form.redirect_uri,
            form.state
                .as_ref()
                .map(|s| format!("&state={}", s))
                .unwrap_or_default()
        );
        return Ok(Redirect::to(&error_redirect).into_response());
    }

    // TODO: Verify client_id exists in database
    // TODO: Verify redirect_uri matches registered URIs

    // Generate authorization code
    let code = generate_authorization_code();

    // TODO: Store authorization code in database with:
    // - code
    // - client_id
    // - redirect_uri
    // - code_challenge
    // - resource
    // - scopes
    // - expires_at (10 minutes from now)

    // Redirect to client with authorization code
    let success_redirect = format!(
        "{}?code={}{}",
        form.redirect_uri,
        code,
        form.state
            .as_ref()
            .map(|s| format!("&state={}", s))
            .unwrap_or_default()
    );

    Ok(Redirect::to(&success_redirect).into_response())
}

/// Validate authorization request parameters
fn validate_authorize_request(params: &AuthorizeRequest) -> Result<(), (StatusCode, Json<OAuthError>)> {
    // Validate response_type
    if params.response_type != "code" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(OAuthError::invalid_request(
                "response_type must be 'code'",
            )),
        ));
    }

    // Validate code_challenge_method (OAuth 2.1 mandates S256)
    if params.code_challenge_method != "S256" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(OAuthError::invalid_request(
                "code_challenge_method must be 'S256'",
            )),
        ));
    }

    // Validate code_challenge format
    if !validate_code_challenge(&params.code_challenge) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(OAuthError::invalid_request(
                "Invalid code_challenge format",
            )),
        ));
    }

    // Validate redirect_uri format
    if !params.redirect_uri.starts_with("https://")
        && !params.redirect_uri.starts_with("http://localhost")
        && !params.redirect_uri.starts_with("http://127.0.0.1")
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(OAuthError::invalid_request(
                "redirect_uri must be HTTPS or http://localhost",
            )),
        ));
    }

    Ok(())
}

/// Render HTML consent form
fn render_consent_form(params: &AuthorizeRequest) -> String {
    let scopes = params
        .scope
        .as_ref()
        .map(|s| s.split_whitespace().collect::<Vec<_>>())
        .unwrap_or_default();

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Authorization Request</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 500px; margin: 50px auto; padding: 20px; }}
        .consent-box {{ border: 1px solid #ccc; padding: 20px; border-radius: 5px; }}
        .scopes {{ margin: 20px 0; }}
        .scope-item {{ padding: 5px 0; }}
        .buttons {{ margin-top: 20px; }}
        button {{ padding: 10px 20px; margin-right: 10px; cursor: pointer; }}
        .approve {{ background-color: #4CAF50; color: white; border: none; }}
        .deny {{ background-color: #f44336; color: white; border: none; }}
    </style>
</head>
<body>
    <div class="consent-box">
        <h2>Authorization Request</h2>
        <p><strong>Client:</strong> {}</p>
        <p><strong>Redirect URI:</strong> {}</p>

        <div class="scopes">
            <p><strong>Requested Permissions:</strong></p>
            {}
        </div>

        <form method="POST" action="/oauth/authorize">
            <input type="hidden" name="client_id" value="{}">
            <input type="hidden" name="redirect_uri" value="{}">
            <input type="hidden" name="code_challenge" value="{}">
            {}
            {}
            {}

            <div class="buttons">
                <button type="submit" name="approved" value="true" class="approve">Approve</button>
                <button type="submit" name="approved" value="false" class="deny">Deny</button>
            </div>
        </form>
    </div>
</body>
</html>"#,
        params.client_id,
        params.redirect_uri,
        if scopes.is_empty() {
            "<p>No specific permissions requested</p>".to_string()
        } else {
            scopes
                .iter()
                .map(|s| format!("<div class='scope-item'>• {}</div>", s))
                .collect::<Vec<_>>()
                .join("\n")
        },
        params.client_id,
        params.redirect_uri,
        params.code_challenge,
        params
            .state
            .as_ref()
            .map(|s| format!(r#"<input type="hidden" name="state" value="{}">"#, s))
            .unwrap_or_default(),
        params
            .resource
            .as_ref()
            .map(|r| format!(r#"<input type="hidden" name="resource" value="{}">"#, r))
            .unwrap_or_default(),
        params
            .scope
            .as_ref()
            .map(|s| format!(r#"<input type="hidden" name="scope" value="{}">"#, s))
            .unwrap_or_default(),
    )
}

/// Generate cryptographically secure authorization code
fn generate_authorization_code() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();

    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

// Re-export Json for error responses
use axum::Json;
