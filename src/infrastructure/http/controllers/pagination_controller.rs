//! HTTP controller for paginated endpoints

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::{
    PostgreSQL,
    errors::AppError,
    infrastructure::errors::ErrorTranslator,
    services::{
        SearchService, UserSearchFilters,
        pagination_service::{PaginationParams, PaginatedResponse},
    },
    models::UserStatus,
};

/// Query parameters for paginated user search
#[derive(Debug, Deserialize)]
pub struct PaginatedSearchQuery {
    // Search term
    pub q: Option<String>,

    // Filters
    pub status: Option<String>,
    pub email_verified: Option<bool>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
    pub location: Option<String>,

    // Pagination
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
    pub direction: Option<String>,

    // Sorting
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

impl From<PaginatedSearchQuery> for PaginationParams {
    fn from(query: PaginatedSearchQuery) -> Self {
        use crate::services::pagination_service::{PaginationDirection, SortOrder};

        PaginationParams {
            page: query.page,
            per_page: query.per_page,
            offset: query.offset,
            limit: query.limit,
            cursor: query.cursor,
            direction: query.direction.and_then(|d| match d.as_str() {
                "forward" => Some(PaginationDirection::Forward),
                "backward" => Some(PaginationDirection::Backward),
                _ => None,
            }),
            sort_by: query.sort_by,
            sort_order: query.sort_order.and_then(|o| match o.as_str() {
                "asc" => Some(SortOrder::Asc),
                "desc" => Some(SortOrder::Desc),
                _ => None,
            }),
        }
    }
}

/// Get users with pagination (supports multiple pagination strategies)
///
/// ## Pagination Strategies
///
/// ### 1. Page-based pagination
/// ```text
/// GET /api/users/paginated?page=2&per_page=20
/// ```
///
/// ### 2. Offset-based pagination
/// ```text
/// GET /api/users/paginated?offset=40&limit=20
/// ```
///
/// ### 3. Cursor-based pagination
/// ```text
/// GET /api/users/paginated?cursor=eyJpZCI6IjU1MGU4NDAwLWUyOWItNDFkNC1hNzE2LTQ0NjY1NTQ0MDAwMCIsInRpbWVzdGFtcCI6IjIwMjQtMDEtMDFUMDA6MDA6MDBaIn0=&direction=forward
/// ```
pub async fn get_users_paginated(
    Query(query): Query<PaginatedSearchQuery>,
    State(db): State<PostgreSQL>,
) -> impl IntoResponse {
    info!("Fetching paginated users with query: {:?}", query);

    let search_service = SearchService::new(db.pool(), db.clone());

    // Convert query to filters
    let filters = UserSearchFilters {
        search: query.q.clone(),
        status: query.status.as_ref().and_then(|s| match s.as_str() {
            "active" => Some(UserStatus::Active),
            "inactive" => Some(UserStatus::Inactive),
            "suspended" => Some(UserStatus::Suspended),
            "pending" => Some(UserStatus::PendingVerification),
            _ => None,
        }),
        email_verified: query.email_verified,
        created_after: query.created_after.as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc)),
        created_before: query.created_before.as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc)),
        location: query.location.clone(),
        sort_by: None, // Handled by pagination params
        sort_desc: None, // Handled by pagination params
        limit: None, // Handled by pagination params
        offset: None, // Handled by pagination params
    };

    // Convert to pagination params
    let pagination = query.into();

    match search_service.search_users_paginated(None, filters, pagination).await {
        Ok(result) => {
            info!("Successfully fetched {} users", result.data.len());
            Json(result).into_response()
        }
        Err(e) => {
            error!("Failed to fetch paginated users: {:?}", e);
            ErrorTranslator::to_http_response(e)
        }
    }
}

/// Response format for the pagination demo endpoint
#[derive(Debug, Serialize)]
pub struct PaginationDemoResponse {
    pub strategies: Vec<PaginationStrategy>,
    pub examples: PaginationExamples,
}

#[derive(Debug, Serialize)]
pub struct PaginationStrategy {
    pub name: String,
    pub description: String,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub use_cases: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PaginationExamples {
    pub page_based: Vec<String>,
    pub offset_based: Vec<String>,
    pub cursor_based: Vec<String>,
}

/// Get pagination documentation and examples
pub async fn get_pagination_demo() -> impl IntoResponse {
    let response = PaginationDemoResponse {
        strategies: vec![
            PaginationStrategy {
                name: "Page-Based Pagination".to_string(),
                description: "Navigate through results using page numbers".to_string(),
                pros: vec![
                    "Easy to understand and implement".to_string(),
                    "Allows jumping to specific pages".to_string(),
                    "Good for small to medium datasets".to_string(),
                ],
                cons: vec![
                    "Can be inefficient for large datasets".to_string(),
                    "Page drift when data changes".to_string(),
                    "Requires counting total records".to_string(),
                ],
                use_cases: vec![
                    "Search results".to_string(),
                    "Admin dashboards".to_string(),
                    "Product catalogs".to_string(),
                ],
            },
            PaginationStrategy {
                name: "Offset-Based Pagination".to_string(),
                description: "Skip a specific number of records".to_string(),
                pros: vec![
                    "Simple to implement".to_string(),
                    "Stateless operation".to_string(),
                    "Works with any ordering".to_string(),
                ],
                cons: vec![
                    "Performance degrades with large offsets".to_string(),
                    "Inconsistent results if data changes".to_string(),
                    "Not suitable for real-time data".to_string(),
                ],
                use_cases: vec![
                    "Data exports".to_string(),
                    "Batch processing".to_string(),
                    "API integrations".to_string(),
                ],
            },
            PaginationStrategy {
                name: "Cursor-Based Pagination".to_string(),
                description: "Use opaque cursors to navigate through results".to_string(),
                pros: vec![
                    "Excellent performance at any position".to_string(),
                    "Stable pagination (no drift)".to_string(),
                    "Works well with real-time data".to_string(),
                ],
                cons: vec![
                    "Cannot jump to arbitrary pages".to_string(),
                    "More complex to implement".to_string(),
                    "Cursors can become invalid".to_string(),
                ],
                use_cases: vec![
                    "Social media feeds".to_string(),
                    "Activity streams".to_string(),
                    "Large datasets".to_string(),
                ],
            },
        ],
        examples: PaginationExamples {
            page_based: vec![
                "/api/users/paginated?page=1&per_page=20".to_string(),
                "/api/users/paginated?page=5&per_page=50&sort_by=created_at&sort_order=desc".to_string(),
                "/api/users/paginated?page=2&per_page=10&q=john&status=active".to_string(),
            ],
            offset_based: vec![
                "/api/users/paginated?offset=0&limit=20".to_string(),
                "/api/users/paginated?offset=100&limit=50".to_string(),
                "/api/users/paginated?offset=200&limit=25&email_verified=true".to_string(),
            ],
            cursor_based: vec![
                "/api/users/paginated?cursor=START&direction=forward&limit=20".to_string(),
                "/api/users/paginated?cursor=eyJpZCI6IjEyMyIsInRzIjoiMjAyNC0wMS0wMSJ9&direction=forward".to_string(),
                "/api/users/paginated?cursor=END_CURSOR&direction=backward&limit=10".to_string(),
            ],
        },
    };

    Json(response)
}

/// Get comparison of pagination strategies with performance metrics
pub async fn get_pagination_comparison(
    State(db): State<PostgreSQL>,
) -> impl IntoResponse {
    // Get total user count
    let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&db.pool())
        .await
        .unwrap_or(0);

    #[derive(Serialize)]
    struct ComparisonResponse {
        total_records: i64,
        strategies: Vec<StrategyComparison>,
        recommendations: Vec<String>,
    }

    #[derive(Serialize)]
    struct StrategyComparison {
        strategy: String,
        performance_at_start: String,
        performance_at_middle: String,
        performance_at_end: String,
        memory_usage: String,
        recommended_for: Vec<String>,
    }

    let response = ComparisonResponse {
        total_records: total_count,
        strategies: vec![
            StrategyComparison {
                strategy: "Page-Based".to_string(),
                performance_at_start: "Fast (O(1))".to_string(),
                performance_at_middle: "Moderate (O(n))".to_string(),
                performance_at_end: "Slow (O(n))".to_string(),
                memory_usage: "Low".to_string(),
                recommended_for: vec![
                    format!("Datasets < {} records", 10000),
                    "User-facing search results".to_string(),
                ],
            },
            StrategyComparison {
                strategy: "Offset-Based".to_string(),
                performance_at_start: "Fast (O(1))".to_string(),
                performance_at_middle: "Slow (O(n))".to_string(),
                performance_at_end: "Very Slow (O(n))".to_string(),
                memory_usage: "Low".to_string(),
                recommended_for: vec![
                    "API integrations".to_string(),
                    "Batch exports".to_string(),
                ],
            },
            StrategyComparison {
                strategy: "Cursor-Based".to_string(),
                performance_at_start: "Fast (O(log n))".to_string(),
                performance_at_middle: "Fast (O(log n))".to_string(),
                performance_at_end: "Fast (O(log n))".to_string(),
                memory_usage: "Moderate".to_string(),
                recommended_for: vec![
                    format!("Datasets > {} records", 10000),
                    "Real-time feeds".to_string(),
                    "Infinite scrolling".to_string(),
                ],
            },
        ],
        recommendations: vec![
            format!("With {} total records, cursor-based pagination is recommended for best performance", total_count),
            "Use page-based for user-friendly interfaces where jumping to specific pages is needed".to_string(),
            "Use cursor-based for APIs and mobile apps with infinite scrolling".to_string(),
            "Consider implementing multiple strategies and let clients choose".to_string(),
        ],
    };

    Json(response)
}