//! Comprehensive pagination service supporting multiple pagination strategies
//!
//! This service provides:
//! - Offset/Limit pagination (traditional)
//! - Page-based pagination
//! - Cursor-based pagination (for efficient large datasets)
//! - Keyset pagination (for stable pagination)

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppError;

/// Pagination request parameters that can be used across different endpoints
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationParams {
    /// Page number (1-based) for page-based pagination
    pub page: Option<u32>,

    /// Number of items per page
    pub per_page: Option<u32>,

    /// Offset for offset-based pagination
    pub offset: Option<i64>,

    /// Limit for the number of results
    pub limit: Option<i64>,

    /// Cursor for cursor-based pagination
    pub cursor: Option<String>,

    /// Direction for cursor pagination
    pub direction: Option<PaginationDirection>,

    /// Sorting field
    pub sort_by: Option<String>,

    /// Sort direction
    pub sort_order: Option<SortOrder>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PaginationDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Pagination response metadata
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationMeta {
    /// Total number of items
    pub total_count: i64,

    /// Number of items in current page
    pub count: i32,

    /// Current page number (for page-based)
    pub current_page: Option<u32>,

    /// Total number of pages
    pub total_pages: Option<u32>,

    /// Items per page
    pub per_page: Option<u32>,

    /// Current offset
    pub offset: Option<i64>,

    /// Whether there are more results
    pub has_next_page: bool,

    /// Whether there are previous results
    pub has_previous_page: bool,

    /// Cursor for the next page
    pub next_cursor: Option<String>,

    /// Cursor for the previous page
    pub prev_cursor: Option<String>,

    /// Start cursor for the current page
    pub start_cursor: Option<String>,

    /// End cursor for the current page
    pub end_cursor: Option<String>,
}

/// Cursor data structure for cursor-based pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    /// Primary key value (usually ID)
    pub id: Uuid,

    /// Timestamp for ordering
    pub timestamp: DateTime<Utc>,

    /// Additional sort key if needed
    pub sort_key: Option<String>,
}

impl Cursor {
    /// Create a new cursor
    pub fn new(id: Uuid, timestamp: DateTime<Utc>) -> Self {
        Self {
            id,
            timestamp,
            sort_key: None,
        }
    }

    /// Create a cursor with a sort key
    pub fn with_sort_key(id: Uuid, timestamp: DateTime<Utc>, sort_key: String) -> Self {
        Self {
            id,
            timestamp,
            sort_key: Some(sort_key),
        }
    }

    /// Encode cursor to base64 string
    pub fn encode(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        BASE64.encode(json.as_bytes())
    }

    /// Decode cursor from base64 string
    pub fn decode(encoded: &str) -> Result<Self, AppError> {
        let decoded = BASE64.decode(encoded)
            .map_err(|_| AppError::Validation {
                field: "cursor".to_string(),
                message: "Invalid cursor format".to_string(),
            })?;

        let cursor_str = String::from_utf8(decoded)
            .map_err(|_| AppError::Validation {
                field: "cursor".to_string(),
                message: "Invalid cursor encoding".to_string(),
            })?;

        serde_json::from_str(&cursor_str)
            .map_err(|_| AppError::Validation {
                field: "cursor".to_string(),
                message: "Invalid cursor data".to_string(),
            })
    }
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T> {
    /// The actual data items
    pub data: Vec<T>,

    /// Pagination metadata
    pub meta: PaginationMeta,

    /// Optional links for HATEOAS
    pub links: Option<PaginationLinks>,
}

/// HATEOAS links for pagination
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationLinks {
    pub first: Option<String>,
    pub prev: Option<String>,
    pub next: Option<String>,
    pub last: Option<String>,
}

/// Pagination service for handling different pagination strategies
pub struct PaginationService;

impl PaginationService {
    /// Convert generic pagination params to offset/limit
    pub fn to_offset_limit(params: &PaginationParams) -> (i64, i64) {
        // Priority: offset/limit > page/per_page > defaults
        if let (Some(offset), Some(limit)) = (params.offset, params.limit) {
            return (offset, limit.min(100)); // Cap at 100
        }

        if let (Some(page), Some(per_page)) = (params.page, params.per_page) {
            let per_page = per_page.min(100) as i64; // Cap at 100
            let offset = ((page.saturating_sub(1)) as i64) * per_page;
            return (offset, per_page);
        }

        // Defaults
        let limit = params.limit.unwrap_or(20).min(100);
        let offset = params.offset.unwrap_or(0);
        (offset, limit)
    }

    /// Calculate pagination metadata
    pub fn calculate_meta(
        total_count: i64,
        offset: i64,
        limit: i64,
        params: &PaginationParams,
    ) -> PaginationMeta {
        let count = limit.min(total_count - offset).max(0) as i32;

        // Calculate page-based metadata if applicable
        let (current_page, total_pages, per_page) = if let Some(page) = params.page {
            let per_page = params.per_page.unwrap_or(20);
            let total_pages = ((total_count as f64) / (per_page as f64)).ceil() as u32;
            (Some(page), Some(total_pages), Some(per_page))
        } else {
            (None, None, None)
        };

        PaginationMeta {
            total_count,
            count,
            current_page,
            total_pages,
            per_page,
            offset: Some(offset),
            has_next_page: offset + limit < total_count,
            has_previous_page: offset > 0,
            next_cursor: None, // Will be set if using cursor pagination
            prev_cursor: None,
            start_cursor: None,
            end_cursor: None,
        }
    }

    /// Build SQL ORDER BY clause from pagination params
    pub fn build_order_clause(params: &PaginationParams, default_field: &str) -> String {
        let field = params.sort_by.as_deref().unwrap_or(default_field);
        let order = match params.sort_order {
            Some(SortOrder::Desc) => "DESC",
            _ => "ASC",
        };

        // Sanitize field name to prevent SQL injection
        let safe_field = Self::sanitize_field_name(field);
        format!("{} {}", safe_field, order)
    }

    /// Build cursor-based WHERE clause
    pub fn build_cursor_clause(
        cursor: Option<&Cursor>,
        direction: &PaginationDirection,
        sort_field: &str,
    ) -> Result<String, AppError> {
        let cursor = match cursor {
            Some(c) => c,
            None => return Ok(String::new()),
        };

        let operator = match direction {
            PaginationDirection::Forward => ">",
            PaginationDirection::Backward => "<",
        };

        // Build WHERE clause based on sort field and cursor
        let clause = match sort_field {
            "created_at" => {
                format!(
                    "(created_at, id) {} ('{}', '{}')",
                    operator,
                    cursor.timestamp.to_rfc3339(),
                    cursor.id
                )
            }
            "username" => {
                let sort_key = cursor.sort_key.as_ref()
                    .ok_or_else(|| AppError::Validation {
                        field: "cursor".to_string(),
                        message: "Missing sort key in cursor".to_string(),
                    })?;
                format!(
                    "(username, id) {} ('{}', '{}')",
                    operator,
                    sort_key,
                    cursor.id
                )
            }
            _ => {
                format!("id {} '{}'", operator, cursor.id)
            }
        };

        Ok(clause)
    }

    /// Create cursor from entity
    pub fn create_cursor<T>(entity: &T, sort_field: &str) -> Cursor
    where
        T: HasCursorFields,
    {
        let mut cursor = Cursor::new(entity.get_id(), entity.get_created_at());

        if sort_field == "username" {
            cursor.sort_key = Some(entity.get_sort_key());
        }

        cursor
    }

    /// Sanitize field name to prevent SQL injection
    fn sanitize_field_name(field: &str) -> String {
        // Whitelist of allowed field names
        match field {
            "id" | "username" | "email" | "created_at" | "updated_at" |
            "last_login_at" | "status" | "email_verified" => field.to_string(),
            _ => "created_at".to_string(), // Default to safe field
        }
    }

    /// Build HATEOAS links
    pub fn build_links(
        base_url: &str,
        params: &PaginationParams,
        meta: &PaginationMeta,
    ) -> PaginationLinks {
        let mut links = PaginationLinks {
            first: None,
            prev: None,
            next: None,
            last: None,
        };

        if let (Some(current_page), Some(total_pages)) = (meta.current_page, meta.total_pages) {
            // Page-based links
            if current_page > 1 {
                links.first = Some(format!("{}?page=1&per_page={}", base_url, meta.per_page.unwrap_or(20)));
                links.prev = Some(format!("{}?page={}&per_page={}", base_url, current_page - 1, meta.per_page.unwrap_or(20)));
            }

            if current_page < total_pages {
                links.next = Some(format!("{}?page={}&per_page={}", base_url, current_page + 1, meta.per_page.unwrap_or(20)));
                links.last = Some(format!("{}?page={}&per_page={}", base_url, total_pages, meta.per_page.unwrap_or(20)));
            }
        } else if let Some(offset) = meta.offset {
            // Offset-based links
            let limit = params.limit.unwrap_or(20);

            if offset > 0 {
                links.first = Some(format!("{}?offset=0&limit={}", base_url, limit));
                let prev_offset = (offset - limit).max(0);
                links.prev = Some(format!("{}?offset={}&limit={}", base_url, prev_offset, limit));
            }

            if meta.has_next_page {
                links.next = Some(format!("{}?offset={}&limit={}", base_url, offset + limit, limit));
            }
        } else if meta.next_cursor.is_some() || meta.prev_cursor.is_some() {
            // Cursor-based links
            if let Some(ref prev_cursor) = meta.prev_cursor {
                links.prev = Some(format!("{}?cursor={}&direction=backward", base_url, prev_cursor));
            }

            if let Some(ref next_cursor) = meta.next_cursor {
                links.next = Some(format!("{}?cursor={}&direction=forward", base_url, next_cursor));
            }
        }

        links
    }
}

/// Trait for entities that can be used with cursor pagination
pub trait HasCursorFields {
    fn get_id(&self) -> Uuid;
    fn get_created_at(&self) -> DateTime<Utc>;
    fn get_sort_key(&self) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_encoding() {
        let cursor = Cursor::new(
            Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            Utc::now(),
        );

        let encoded = cursor.encode();
        let decoded = Cursor::decode(&encoded).unwrap();

        assert_eq!(cursor.id, decoded.id);
    }

    #[test]
    fn test_offset_limit_conversion() {
        // Test with offset and limit
        let params = PaginationParams {
            offset: Some(10),
            limit: Some(20),
            page: None,
            per_page: None,
            cursor: None,
            direction: None,
            sort_by: None,
            sort_order: None,
        };

        let (offset, limit) = PaginationService::to_offset_limit(&params);
        assert_eq!(offset, 10);
        assert_eq!(limit, 20);

        // Test with page and per_page
        let params = PaginationParams {
            offset: None,
            limit: None,
            page: Some(3),
            per_page: Some(25),
            cursor: None,
            direction: None,
            sort_by: None,
            sort_order: None,
        };

        let (offset, limit) = PaginationService::to_offset_limit(&params);
        assert_eq!(offset, 50); // (3-1) * 25
        assert_eq!(limit, 25);
    }

    #[test]
    fn test_pagination_meta_calculation() {
        let params = PaginationParams {
            page: Some(2),
            per_page: Some(10),
            offset: None,
            limit: None,
            cursor: None,
            direction: None,
            sort_by: None,
            sort_order: None,
        };

        let meta = PaginationService::calculate_meta(95, 10, 10, &params);

        assert_eq!(meta.total_count, 95);
        assert_eq!(meta.current_page, Some(2));
        assert_eq!(meta.total_pages, Some(10));
        assert_eq!(meta.has_next_page, true);
        assert_eq!(meta.has_previous_page, true);
    }
}