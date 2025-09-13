    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<models::User>, sqlx::Error> {
        sqlx::query_as!(
            models::User,
            r#"
            SELECT id, username, email, password_hash, email_verified, status as "status: models::UserStatus",
                   created_at, updated_at, last_login_at
            FROM users WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.db)
        .await
    }