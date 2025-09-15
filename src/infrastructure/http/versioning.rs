use axum::{
    extract::{Request, Path},
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use chrono::{DateTime, Utc};

/// Supported API versions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ApiVersion {
    V1,
    V2,
    V3,
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiVersion::V1 => write!(f, "v1"),
            ApiVersion::V2 => write!(f, "v2"),
            ApiVersion::V3 => write!(f, "v3"),
        }
    }
}

impl ApiVersion {
    /// Parse version from string (e.g., "v1", "v2", "1", "2")
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "v1" | "1" => Some(ApiVersion::V1),
            "v2" | "2" => Some(ApiVersion::V2),
            "v3" | "3" => Some(ApiVersion::V3),
            _ => None,
        }
    }

    /// Get the current/latest supported version
    pub fn current() -> Self {
        ApiVersion::V3
    }

    /// Check if this version is deprecated
    pub fn is_deprecated(&self) -> bool {
        match self {
            ApiVersion::V1 => true,  // V1 is deprecated
            ApiVersion::V2 => false, // V2 is current stable
            ApiVersion::V3 => false, // V3 is latest
        }
    }

    /// Get deprecation information if applicable
    pub fn deprecation_info(&self) -> Option<DeprecationInfo> {
        match self {
            ApiVersion::V1 => Some(DeprecationInfo {
                deprecated_since: "2025-01-01T00:00:00Z".parse().ok()?,
                sunset_date: Some("2025-06-01T00:00:00Z".parse().ok()?),
                replacement_version: Some(ApiVersion::V2),
                message: "API v1 is deprecated. Please migrate to v2 for improved features and security.".to_string(),
            }),
            ApiVersion::V2 | ApiVersion::V3 => None,
        }
    }

    /// Get all supported versions
    pub fn all() -> Vec<Self> {
        vec![ApiVersion::V1, ApiVersion::V2, ApiVersion::V3]
    }
}

/// Information about deprecated API versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprecationInfo {
    pub deprecated_since: DateTime<Utc>,
    pub sunset_date: Option<DateTime<Utc>>,
    pub replacement_version: Option<ApiVersion>,
    pub message: String,
}

/// Extract API version from request
pub fn extract_version(request: &Request) -> ApiVersion {
    // Try to get version from URL path (e.g., /api/v1/users)
    if let Some(path_version) = extract_version_from_path(request.uri().path()) {
        return path_version;
    }

    // Try to get version from Accept header (e.g., application/vnd.api+json;version=1)
    if let Some(header_version) = extract_version_from_headers(request.headers()) {
        return header_version;
    }

    // Try to get version from custom header (e.g., X-API-Version: v2)
    if let Some(custom_header_version) = extract_version_from_custom_header(request.headers()) {
        return custom_header_version;
    }

    // Default to current version
    ApiVersion::current()
}

fn extract_version_from_path(path: &str) -> Option<ApiVersion> {
    // Match patterns like /api/v1/, /api/v2/, etc.
    let path_segments: Vec<&str> = path.split('/').collect();
    for segment in path_segments {
        if let Some(version) = ApiVersion::from_str(segment) {
            return Some(version);
        }
    }
    None
}

fn extract_version_from_headers(headers: &HeaderMap) -> Option<ApiVersion> {
    // Look for Accept header with version specification
    // e.g., "application/vnd.api+json;version=2"
    if let Some(accept) = headers.get(header::ACCEPT) {
        if let Ok(accept_str) = accept.to_str() {
            if let Some(version_part) = accept_str.split(';').find(|part| part.trim().starts_with("version=")) {
                if let Some(version_str) = version_part.split('=').nth(1) {
                    return ApiVersion::from_str(version_str.trim());
                }
            }
        }
    }
    None
}

fn extract_version_from_custom_header(headers: &HeaderMap) -> Option<ApiVersion> {
    // Look for custom X-API-Version header
    if let Some(version_header) = headers.get("X-API-Version") {
        if let Ok(version_str) = version_header.to_str() {
            return ApiVersion::from_str(version_str);
        }
    }
    None
}

/// Middleware to handle API versioning and deprecation warnings
pub async fn versioning_middleware(mut request: Request, next: Next) -> Response {
    let version = extract_version(&request);

    // Add version to request extensions for handlers to access
    request.extensions_mut().insert(version);

    // Process the request
    let mut response = next.run(request).await;

    // Add version headers to response
    let headers = response.headers_mut();
    headers.insert("X-API-Version", version.to_string().parse().unwrap());
    headers.insert("X-API-Supported-Versions",
        ApiVersion::all().iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",").parse().unwrap());

    // Add deprecation warnings if applicable
    if let Some(deprecation_info) = version.deprecation_info() {
        // Add deprecation header as per RFC 8594
        headers.insert("Deprecation", deprecation_info.deprecated_since.to_rfc3339().parse().unwrap());

        if let Some(sunset) = deprecation_info.sunset_date {
            headers.insert("Sunset", sunset.to_rfc3339().parse().unwrap());
        }

        // Add warning header as per RFC 7234
        let warning = format!("299 - \"{}\"", deprecation_info.message);
        headers.insert("Warning", warning.parse().unwrap());

        // Add link to newer version if available
        if let Some(replacement) = deprecation_info.replacement_version {
            let link = format!("<{}>; rel=\"successor-version\"", replacement);
            headers.insert("Link", link.parse().unwrap());
        }
    }

    response
}

/// Response wrapper for versioned APIs
#[derive(Debug, Serialize)]
pub struct VersionedResponse<T> {
    pub api_version: ApiVersion,
    pub data: T,
    pub meta: ResponseMeta,
}

#[derive(Debug, Serialize)]
pub struct ResponseMeta {
    pub timestamp: DateTime<Utc>,
    pub deprecation_info: Option<DeprecationInfo>,
}

impl<T> VersionedResponse<T> {
    pub fn new(data: T, version: ApiVersion) -> Self {
        Self {
            api_version: version,
            data,
            meta: ResponseMeta {
                timestamp: Utc::now(),
                deprecation_info: version.deprecation_info(),
            },
        }
    }
}

/// Helper to create versioned JSON response
pub fn versioned_json<T: Serialize>(data: T, version: ApiVersion) -> impl IntoResponse {
    let response = VersionedResponse::new(data, version);
    Json(response)
}

/// Version-specific route handler
pub async fn api_info(version: Option<Path<String>>) -> impl IntoResponse {
    let requested_version = version
        .as_ref()
        .and_then(|v| ApiVersion::from_str(v))
        .unwrap_or(ApiVersion::current());

    let info = serde_json::json!({
        "api_name": "User Management API",
        "version": requested_version,
        "current_version": ApiVersion::current(),
        "supported_versions": ApiVersion::all(),
        "deprecation_info": requested_version.deprecation_info(),
        "endpoints": {
            "authentication": format!("/api/{}/auth", requested_version),
            "users": format!("/api/{}/users", requested_version),
            "profiles": format!("/api/{}/profiles", requested_version),
            "admin": format!("/api/{}/admin", requested_version),
        },
        "documentation": format!("https://api-docs.example.com/{}", requested_version),
    });

    versioned_json(info, requested_version)
}

/// Handler for unsupported API versions
pub async fn unsupported_version_handler() -> impl IntoResponse {
    let error_response = serde_json::json!({
        "error": "Unsupported API Version",
        "message": "The requested API version is not supported",
        "supported_versions": ApiVersion::all(),
        "current_version": ApiVersion::current(),
    });

    (StatusCode::BAD_REQUEST, Json(error_response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        assert_eq!(ApiVersion::from_str("v1"), Some(ApiVersion::V1));
        assert_eq!(ApiVersion::from_str("V2"), Some(ApiVersion::V2));
        assert_eq!(ApiVersion::from_str("3"), Some(ApiVersion::V3));
        assert_eq!(ApiVersion::from_str("invalid"), None);
    }

    #[test]
    fn test_version_ordering() {
        assert!(ApiVersion::V1 < ApiVersion::V2);
        assert!(ApiVersion::V2 < ApiVersion::V3);
    }

    #[test]
    fn test_deprecation() {
        assert!(ApiVersion::V1.is_deprecated());
        assert!(!ApiVersion::V2.is_deprecated());
        assert!(!ApiVersion::V3.is_deprecated());
    }

    #[test]
    fn test_path_extraction() {
        assert_eq!(extract_version_from_path("/api/v1/users"), Some(ApiVersion::V1));
        assert_eq!(extract_version_from_path("/api/v2/profiles"), Some(ApiVersion::V2));
        assert_eq!(extract_version_from_path("/api/users"), None);
    }
}