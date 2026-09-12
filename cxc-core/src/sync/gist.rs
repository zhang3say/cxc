use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

const GITHUB_API_BASE: &str = "https://api.github.com";
const GIST_FILENAME: &str = "cxc_sync.json";
const GIST_DESCRIPTION: &str = "CXC (Code Cross-Connect) Configuration Sync";

#[derive(Error, Debug)]
pub enum GistError {
    #[error("Network request failed: {0}")]
    Network(#[from] reqwest::Error),
    #[error("GitHub API error: status {status}, message: {message}")]
    Api { status: u16, message: String },
    #[error("Gist file '{0}' not found in gist")]
    FileNotFound(String),
    #[error("Failed to parse GitHub response: {0}")]
    Parse(String),
}

#[derive(Serialize, Deserialize, Debug)]
struct GitHubUserResponse {
    pub login: String,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GistFilePayload {
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateGistRequest {
    pub description: String,
    pub public: bool,
    pub files: HashMap<String, GistFilePayload>,
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateGistRequest {
    pub description: String,
    pub files: HashMap<String, GistFilePayload>,
}

#[derive(Deserialize, Debug)]
struct GistFileResponse {
    #[allow(dead_code)]
    pub filename: Option<String>,
    pub content: Option<String>,
    #[serde(default)]
    pub truncated: bool,
    pub raw_url: Option<String>,
}

#[derive(Deserialize, Debug)]
struct GistResponse {
    pub id: String,
    pub files: HashMap<String, GistFileResponse>,
}

pub struct GistClient {
    client: reqwest::Client,
    token: String,
}

impl GistClient {
    pub fn new(token: String) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(concat!("CXC-Desktop/", env!("CARGO_PKG_VERSION"), " (https://github.com/zhang3say/cxc)")),
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_static("2022-11-28"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_default();

        Self { client, token }
    }

    /// Validate the personal access token by querying /user.
    /// Returns the authenticated GitHub username.
    pub async fn validate_token(&self) -> Result<String, GistError> {
        let url = format!("{}/user", GITHUB_API_BASE);
        let resp = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token.trim()))
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(GistError::Api {
                status: status.as_u16(),
                message: body,
            });
        }

        let user: GitHubUserResponse = resp.json().await.map_err(|e| GistError::Parse(e.to_string()))?;
        Ok(user.login)
    }

    /// Creates a new secret Gist containing `content` and returns the new Gist ID.
    pub async fn create_sync_gist(&self, content: &str) -> Result<String, GistError> {
        let url = format!("{}/gists", GITHUB_API_BASE);

        let mut files = HashMap::new();
        files.insert(
            GIST_FILENAME.to_string(),
            GistFilePayload {
                content: content.to_string(),
            },
        );

        let payload = CreateGistRequest {
            description: GIST_DESCRIPTION.to_string(),
            public: false,
            files,
        };

        let resp = self
            .client
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token.trim()))
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(GistError::Api {
                status: status.as_u16(),
                message: body,
            });
        }

        let gist: GistResponse = resp.json().await.map_err(|e| GistError::Parse(e.to_string()))?;
        Ok(gist.id)
    }

    /// Fetches the sync file content from an existing Gist.
    pub async fn get_sync_gist(&self, gist_id: &str) -> Result<String, GistError> {
        let url = format!("{}/gists/{}", GITHUB_API_BASE, gist_id.trim());
        let resp = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token.trim()))
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(GistError::Api {
                status: status.as_u16(),
                message: body,
            });
        }

        let gist: GistResponse = resp.json().await.map_err(|e| GistError::Parse(e.to_string()))?;

        // Look for the cxc_sync.json file, or fall back to the first available file
        let file = gist
            .files
            .get(GIST_FILENAME)
            .or_else(|| gist.files.values().next())
            .ok_or_else(|| GistError::FileNotFound(GIST_FILENAME.to_string()))?;

        if let Some(content) = &file.content {
            if !file.truncated {
                return Ok(content.clone());
            }
        }

        // If content was truncated, download raw content via raw_url
        if let Some(raw_url) = &file.raw_url {
            let raw_resp = self
                .client
                .get(raw_url)
                .header(AUTHORIZATION, format!("Bearer {}", self.token.trim()))
                .send()
                .await?;
            if raw_resp.status().is_success() {
                return Ok(raw_resp.text().await?);
            }
        }

        file.content
            .clone()
            .ok_or_else(|| GistError::FileNotFound(GIST_FILENAME.to_string()))
    }

    /// Updates the sync file in an existing Gist with new content.
    pub async fn update_sync_gist(&self, gist_id: &str, content: &str) -> Result<(), GistError> {
        let url = format!("{}/gists/{}", GITHUB_API_BASE, gist_id.trim());

        let mut files = HashMap::new();
        files.insert(
            GIST_FILENAME.to_string(),
            GistFilePayload {
                content: content.to_string(),
            },
        );

        let payload = UpdateGistRequest {
            description: GIST_DESCRIPTION.to_string(),
            files,
        };

        let resp = self
            .client
            .patch(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token.trim()))
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(GistError::Api {
                status: status.as_u16(),
                message: body,
            });
        }

        Ok(())
    }
}
