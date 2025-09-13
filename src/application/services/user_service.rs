    pub async fn handle_create_user(&self, cmd: CreateUser) -> Result<(), sqlx::Error> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(cmd.password.as_bytes(), &salt)
            .map_err(|_| sqlx::Error::RowNotFound)?
            .to_string();

        let now = Utc::now();
        let user = User {
            id: Uuid::now_v7(),
            username: cmd.username,
            email: cmd.email,
            password_hash,
            email_verified: true, // Legacy users are considered verified
            status: UserStatus::Active,
            created_at: now,
            updated_at: now,
            last_login_at: None,
        };

        self.repo.save_user(user).await?;
        Ok(())
    }