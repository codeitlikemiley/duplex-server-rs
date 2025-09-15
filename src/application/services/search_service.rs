use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::UserStatus,
    infrastructure::repositories::PostgreSQL,
    services::pagination_service::{
        PaginationParams, PaginationService, PaginatedResponse,
        Cursor, PaginationDirection,
    },
};

pub struct SearchService {
    db: Pool<Postgres>,
    postgres: PostgreSQL,
}

impl SearchService {
    pub fn new(pool: Pool<Postgres>, postgres: PostgreSQL) -> Self {
        Self { db: pool, postgres }
    }

    /// Search users with paginated results using the new pagination system
    pub async fn search_users_paginated(
        &self,
        search_term: Option<String>,
        filters: UserSearchFilters,
        pagination: PaginationParams,
    ) -> Result<PaginatedResponse<PublicUser>, AppError> {
        // Convert pagination params to offset/limit
        let (offset, limit) = PaginationService::to_offset_limit(&pagination);

        // Build base query
        let mut query = String::from(
            "SELECT u.*, p.first_name, p.last_name, p.bio, p.avatar_url, p.website, p.location
             FROM users u
             LEFT JOIN user_profiles p ON u.id = p.user_id"
        );

        let mut conditions = Vec::new();
        let mut bind_values: Vec<String> = Vec::new();

        // Add search term if provided
        if let Some(ref term) = search_term {
            conditions.push(
                "(u.username ILIKE $1 OR u.email ILIKE $1 OR p.first_name ILIKE $1 OR p.last_name ILIKE $1)"
            );
            bind_values.push(format!("%{}%", term));
        }

        // Add status filter
        if let Some(ref status) = filters.status {
            conditions.push("u.status = $2");
            bind_values.push(format!("{:?}", status));
        }

        // Add WHERE clause if conditions exist
        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        // Handle cursor-based pagination if cursor is provided
        if let Some(ref cursor_str) = pagination.cursor {
            let cursor = Cursor::decode(cursor_str)?;
            let direction = pagination.direction.as_ref().unwrap_or(&PaginationDirection::Forward);
            let cursor_clause = PaginationService::build_cursor_clause(
                Some(&cursor),
                direction,
                pagination.sort_by.as_deref().unwrap_or("created_at"),
            )?;

            if !cursor_clause.is_empty() {
                if conditions.is_empty() {
                    query.push_str(" WHERE ");
                } else {
                    query.push_str(" AND ");
                }
                query.push_str(&cursor_clause);
            }
        }

        // Add ordering
        let order_clause = PaginationService::build_order_clause(&pagination, "created_at");
        query.push_str(" ORDER BY ");
        query.push_str(&order_clause);

        // Add limit and offset
        query.push_str(&format!(" LIMIT {} OFFSET {}", limit + 1, offset)); // +1 to check if there's next page

        // Execute main query
        let rows = sqlx::query_as::<_, UserWithProfile>(&query)
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::Database {
                message: format!("Failed to search users: {}", e),
            })?;

        // Check if we have more results than requested (for has_next_page)
        let has_more = rows.len() > limit as usize;
        let users: Vec<PublicUser> = rows
            .into_iter()
            .take(limit as usize) // Take only requested amount
            .map(|row| row.into())
            .collect();

        // Get total count (separate query for efficiency)
        let count_query = "SELECT COUNT(*) FROM users u LEFT JOIN user_profiles p ON u.id = p.user_id";
        let total_count: i64 = sqlx::query_scalar(count_query)
            .fetch_one(&self.db)
            .await
            .map_err(|e| AppError::Database {
                message: format!("Failed to count users: {}", e),
            })?;

        // Calculate metadata
        let mut meta = PaginationService::calculate_meta(total_count, offset, limit, &pagination);

        // Add cursor information if using cursor pagination
        if !users.is_empty() && pagination.cursor.is_some() {
            let first_user = &users[0];
            let last_user = &users[users.len() - 1];

            // Create cursors for navigation
            let start_cursor = Cursor::new(first_user.id, first_user.created_at).encode();
            let end_cursor = Cursor::new(last_user.id, last_user.created_at).encode();

            meta.start_cursor = Some(start_cursor.clone());
            meta.end_cursor = Some(end_cursor.clone());

            if has_more {
                meta.next_cursor = Some(end_cursor);
            }

            if offset > 0 {
                meta.prev_cursor = Some(start_cursor);
            }
        }

        // Build HATEOAS links
        let links = Some(PaginationService::build_links("/api/users", &pagination, &meta));

        Ok(PaginatedResponse {
            data: users,
            meta,
            links,
        })
    }

    /// Search users with various filters (legacy method)
    pub async fn search_users(&self, filters: UserSearchFilters) -> Result<UserSearchResult, AppError> {
        let mut query = String::from(
            "SELECT u.id, u.username, u.email, u.password_hash, u.email_verified,
                    u.status as \"status: UserStatus\", u.created_at, u.updated_at, u.last_login_at,
                    p.first_name, p.last_name, p.bio, p.avatar_url, p.website, p.location
             FROM users u
             LEFT JOIN user_profiles p ON u.id = p.user_id"
        );

        let mut conditions = Vec::new();
        let mut args_count = 0;

        // Build WHERE conditions dynamically
        if let Some(ref search_term) = filters.search {
            args_count += 1;
            conditions.push(format!(
                "(u.username ILIKE ${} OR u.email ILIKE ${} OR p.first_name ILIKE ${} OR p.last_name ILIKE ${})",
                args_count, args_count, args_count, args_count
            ));
        }

        if let Some(ref status) = filters.status {
            args_count += 1;
            conditions.push(format!("u.status = ${}", args_count));
        }

        if let Some(email_verified) = filters.email_verified {
            args_count += 1;
            conditions.push(format!("u.email_verified = ${}", args_count));
        }

        if let Some(ref created_after) = filters.created_after {
            args_count += 1;
            conditions.push(format!("u.created_at >= ${}", args_count));
        }

        if let Some(ref created_before) = filters.created_before {
            args_count += 1;
            conditions.push(format!("u.created_at <= ${}", args_count));
        }

        if let Some(ref location) = filters.location {
            args_count += 1;
            conditions.push(format!("p.location ILIKE ${}", args_count));
        }

        // Add WHERE clause if we have conditions
        if !conditions.is_empty() {
            query.push_str(&format!(" WHERE {}", conditions.join(" AND ")));
        }

        // Add ordering
        let order_by = match filters.sort_by.as_deref() {
            Some("username") => "u.username",
            Some("created_at") => "u.created_at",
            Some("last_login_at") => "u.last_login_at",
            Some("email") => "u.email",
            _ => "u.created_at", // default
        };

        let order_direction = if filters.sort_desc.unwrap_or(false) { "DESC" } else { "ASC" };
        query.push_str(&format!(" ORDER BY {} {}", order_by, order_direction));

        // Add pagination
        let limit = filters.limit.unwrap_or(50).min(100); // Max 100 results
        let offset = filters.offset.unwrap_or(0);
        query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        // Build and execute query
        let mut sql_query = sqlx::query_as::<_, UserWithProfile>(&query);

        // Bind parameters in order - need to create patterns outside the binding to avoid lifetime issues
        if let Some(ref search_term) = filters.search {
            let search_pattern = format!("%{}%", search_term);
            sql_query = sql_query.bind(search_pattern);
        }

        if let Some(ref status) = filters.status {
            sql_query = sql_query.bind(status);
        }

        if let Some(email_verified) = filters.email_verified {
            sql_query = sql_query.bind(email_verified);
        }

        if let Some(ref created_after) = filters.created_after {
            sql_query = sql_query.bind(created_after);
        }

        if let Some(ref created_before) = filters.created_before {
            sql_query = sql_query.bind(created_before);
        }

        if let Some(ref location) = filters.location {
            let location_pattern = format!("%{}%", location);
            sql_query = sql_query.bind(location_pattern);
        }

        let users = sql_query
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::Database {
                message: format!("Failed to search users: {}", e)
            })?;

        // Get total count for pagination
        let total_count = self.count_users(&filters).await?;

        Ok(UserSearchResult {
            users: users.into_iter().map(|u| u.into()).collect(),
            total_count,
            limit,
            offset,
            has_more: (offset + limit as i64) < total_count,
        })
    }

    /// Count users matching the filters (for pagination)
    async fn count_users(&self, filters: &UserSearchFilters) -> Result<i64, AppError> {
        let mut query = String::from(
            "SELECT COUNT(DISTINCT u.id) FROM users u LEFT JOIN user_profiles p ON u.id = p.user_id"
        );

        let mut conditions = Vec::new();
        let mut args_count = 0;

        // Build same conditions as search
        if let Some(ref search_term) = filters.search {
            args_count += 1;
            conditions.push(format!(
                "(u.username ILIKE ${} OR u.email ILIKE ${} OR p.first_name ILIKE ${} OR p.last_name ILIKE ${})",
                args_count, args_count, args_count, args_count
            ));
        }

        if let Some(ref status) = filters.status {
            args_count += 1;
            conditions.push(format!("u.status = ${}", args_count));
        }

        if let Some(email_verified) = filters.email_verified {
            args_count += 1;
            conditions.push(format!("u.email_verified = ${}", args_count));
        }

        if let Some(ref created_after) = filters.created_after {
            args_count += 1;
            conditions.push(format!("u.created_at >= ${}", args_count));
        }

        if let Some(ref created_before) = filters.created_before {
            args_count += 1;
            conditions.push(format!("u.created_at <= ${}", args_count));
        }

        if let Some(ref location) = filters.location {
            args_count += 1;
            conditions.push(format!("p.location ILIKE ${}", args_count));
        }

        if !conditions.is_empty() {
            query.push_str(&format!(" WHERE {}", conditions.join(" AND ")));
        }

        let mut sql_query = sqlx::query_scalar::<_, i64>(&query);

        // Bind same parameters
        if let Some(ref search_term) = filters.search {
            let search_pattern = format!("%{}%", search_term);
            sql_query = sql_query.bind(search_pattern);
        }

        if let Some(ref status) = filters.status {
            sql_query = sql_query.bind(status);
        }

        if let Some(email_verified) = filters.email_verified {
            sql_query = sql_query.bind(email_verified);
        }

        if let Some(ref created_after) = filters.created_after {
            sql_query = sql_query.bind(created_after);
        }

        if let Some(ref created_before) = filters.created_before {
            sql_query = sql_query.bind(created_before);
        }

        if let Some(ref location) = filters.location {
            let location_pattern = format!("%{}%", location);
            sql_query = sql_query.bind(location_pattern);
        }

        let count = sql_query
            .fetch_one(&self.db)
            .await
            .map_err(|e| AppError::Database {
                message: format!("Failed to count users: {}", e)
            })?;

        Ok(count)
    }

    /// Get user suggestions based on partial input
    pub async fn suggest_users(&self, query: &str, limit: i32) -> Result<Vec<UserSuggestion>, AppError> {
        let suggestions = sqlx::query_as!(
            UserSuggestion,
            r#"
            SELECT u.id, u.username, u.email, p.first_name, p.last_name, p.avatar_url
            FROM users u
            LEFT JOIN user_profiles p ON u.id = p.user_id
            WHERE (u.username ILIKE $1 OR u.email ILIKE $1 OR
                   p.first_name ILIKE $1 OR p.last_name ILIKE $1)
              AND u.status = 'Active'
              AND u.email_verified = true
            ORDER BY
                CASE
                    WHEN u.username ILIKE $2 THEN 1
                    WHEN u.email ILIKE $2 THEN 2
                    WHEN p.first_name ILIKE $2 OR p.last_name ILIKE $2 THEN 3
                    ELSE 4
                END,
                u.username
            LIMIT $3
            "#,
            format!("%{}%", query),
            format!("{}%", query), // Prefix match gets higher priority
            limit as i64
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to get user suggestions: {}", e)
        })?;

        Ok(suggestions)
    }

    /// Advanced search with full-text search capabilities
    pub async fn advanced_search(&self, filters: AdvancedSearchFilters) -> Result<UserSearchResult, AppError> {
        // This would implement more sophisticated search using PostgreSQL's full-text search
        // For now, we'll use the basic search as a fallback
        let basic_filters = UserSearchFilters {
            search: filters.query,
            status: filters.status,
            email_verified: filters.email_verified,
            created_after: filters.created_after,
            created_before: filters.created_before,
            location: filters.location,
            sort_by: filters.sort_by,
            sort_desc: filters.sort_desc,
            limit: filters.limit,
            offset: filters.offset,
        };

        self.search_users(basic_filters).await
    }
}

// DTOs and Types
#[derive(Debug, serde::Deserialize)]
pub struct UserSearchFilters {
    pub search: Option<String>,
    pub status: Option<UserStatus>,
    pub email_verified: Option<bool>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub location: Option<String>,
    pub sort_by: Option<String>, // username, created_at, last_login_at, email
    pub sort_desc: Option<bool>,
    pub limit: Option<i32>,
    pub offset: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
pub struct AdvancedSearchFilters {
    pub query: Option<String>,
    pub status: Option<UserStatus>,
    pub email_verified: Option<bool>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub location: Option<String>,
    pub has_profile: Option<bool>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
    pub limit: Option<i32>,
    pub offset: Option<i64>,
}

#[derive(Debug, serde::Serialize)]
pub struct UserSearchResult {
    pub users: Vec<PublicUser>,
    pub total_count: i64,
    pub limit: i32,
    pub offset: i64,
    pub has_more: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct PublicUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub profile: Option<PublicProfile>,
}

#[derive(Debug, serde::Serialize)]
pub struct PublicProfile {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub website: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct UserWithProfile {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub email_verified: bool,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub website: Option<String>,
    pub location: Option<String>,
}

impl From<UserWithProfile> for PublicUser {
    fn from(user: UserWithProfile) -> Self {
        let profile = if user.first_name.is_some() || user.last_name.is_some() ||
                        user.bio.is_some() || user.avatar_url.is_some() ||
                        user.website.is_some() || user.location.is_some() {
            Some(PublicProfile {
                first_name: user.first_name,
                last_name: user.last_name,
                bio: user.bio,
                avatar_url: user.avatar_url,
                website: user.website,
                location: user.location,
            })
        } else {
            None
        };

        Self {
            id: user.id,
            username: user.username,
            email: user.email, // Note: In real app, might want to hide emails based on privacy settings
            email_verified: user.email_verified,
            status: user.status,
            created_at: user.created_at,
            last_login_at: user.last_login_at,
            profile,
        }
    }
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct UserSuggestion {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub avatar_url: Option<String>,
}