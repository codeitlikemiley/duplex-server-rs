# Domain Driven Design in Rust

> Using Axum for REST API

### Getting Started

#### Requirements

- postgresql
- rust
- cargo
- sqlx-cli
- cargo-runner


<details>

<summary>Set up Database</summary>

1. add to your ENV to your .bashrc or .zshrc

```sh
export DATABASE_URL="postgres://username@localhost:5432/ddd"
```

we can use cargo-runner to set up ENV, press CMD+SHIFT+R 
then pick env then add `DATABASE_URL="postgres://username@localhost:5432/ddd"`

the next the we run our app with CMD + R it would have that ENV passed on the command

2. Create Database

```sh
sqlx database create
```

3. Create migration

```sh
sqlx migrate add -r <name>
```

Note: if sqlx didnt produce 2 files with *.up.sql and *.down.sql , you need to manually rename and add the down migration

4. Migration

```sh
slqx migrate run
```

5. Prepare Sqlx Compile Time Check 

```sh
cargo sqlx prepare
```
NOTE: you need to add on your vscode settings.json

```json
{
    "rust-analyzer.cargo.extraEnv": {
        "SQLX_OFFLINE": "1",
      },
}
```

everytime you run this there will be new files added in `.sqlx`folder

6. Reset Migration

```sh
sqlx database reset
```

</details>


### DDD Traits

<details>
<summary>1. Command</summary>

<br>

```rust
use serde::{de::DeserializeOwned};

#[allow(dead_code)]
pub trait Command: DeserializeOwned {}
```

</details>

<details>
<summary>2. Events</summary>

<br>

```rust
use serde::{de::DeserializeOwned, ser::Serialize};

#[allow(dead_code)]
pub trait Event: DeserializeOwned + Serialize + Unpin + Send + Sync + 'static {}
```

</details>

<details>
<summary>3. Model/Entity</summary>

<br>

```rust
use serde::{de::DeserializeOwned, ser::Serialize};

#[allow(dead_code)]
pub trait Model: Serialize + DeserializeOwned + Unpin + Send + Sync + 'static {}
```

</details>


### Workflow Rest API

<details>
<summary>1. Create Routes Api</summary>

<br>

```rust
pub enum Api {
    CreateUser,
    GetUser,
}

impl From<Api> for &'static str {
    fn from(value: Api) -> Self {
        match value {
            Api::CreateUser => "/users",
            Api::GetUser => "/users/:id",
        }
    }
}
```

</details>


<details>
<summary>2. Impl Command</summary>

<br>

```rust
use serde::Deserialize;

use crate::domain::Command;

#[derive(Deserialize, Debug)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
}

impl Command for CreateUser {}
```

</details>


<details>
<summary>3. Impl Event</summary>

<br>

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::Event;

#[derive(Serialize, Deserialize, Debug)]
pub struct UserCreated {
    pub id: Uuid,
    pub username: String,
    pub email: String,
}
impl Event for UserCreated {}
```

</details>



<details>
<summary>4. impl Model</summary>

<br>

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::Model;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
}

impl Model for User {}

```

</details>



<details>
<summary>5. Create Repository</summary>

<br>

```rust
use axum::async_trait;
use uuid::Uuid;

use crate::{
    events::UserCreated,
    models::{self},
};

#[async_trait]
pub trait UserRepository {
    async fn save_user(&self, user: models::User) -> Result<(), sqlx::Error>;
    async fn save_event(&self, event: UserCreated) -> Result<(), sqlx::Error>;
    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<models::User>, sqlx::Error>;
}
```

</details>

- Note: you might need to use `axum::async_trait` for async fn

<details>
<summary>6. Impl Repository</summary>

<br>

```rust
use axum::async_trait;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::{events::UserCreated, models, repositories::UserRepository};

#[derive(Clone)]
pub struct PgPool {
    db: Pool<Postgres>,
}

impl PgPool {
    pub fn new(db: Pool<Postgres>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UserRepository for PgPool {
    async fn save_user(&self, user: models::User) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO users (id,username,email) VALUES ($1,$2,$3)",
            user.id,
            &user.username,
            &user.email,
        )
        .execute(&self.db)
        .await?;
        Ok(())
    }

    async fn save_event(&self, event: UserCreated) -> Result<(), sqlx::Error> {
        let payload = serde_json::to_value(&event).unwrap();
        sqlx::query!(
            "INSERT INTO events (id,payload) VALUES ($1,$2)",
            Uuid::now_v7(),
            payload
        )
        .execute(&self.db)
        .await?;
        Ok(())
    }

    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<models::User>, sqlx::Error> {
        sqlx::query_as!(models::User, "SELECT * from users WHERE id = $1", id)
            .fetch_optional(&self.db)
            .await
    }
}
```

</details>

<details><summary>7. SQLX Compile Time Check</summary> 

<br>

Note: This is only required if we use `query!` and `query_as!` macro.

```rust
cargo sqlx prepare
```

</details>


<details>
<summary>7. Create Service Provider</summary>

<br>

```rust
use uuid::Uuid;

use crate::{
    commands::CreateUser,
    db,
    events::UserCreated,
    models::{self},
    repositories::UserRepository,
};

#[derive(Clone)]
pub struct UserService {
    pub repo: db::PgPool,
}

impl UserService {
    pub fn new(repo: db::PgPool) -> Self {
        Self { repo }
    }

    pub async fn handle_create_user(&self, cmd: CreateUser) -> Result<(), sqlx::Error> {
        let user = models::User {
            id: Uuid::now_v7(),
            username: cmd.username,
            email: cmd.email,
        };

        let event = UserCreated {
            id: user.id,
            username: user.username.clone(),
            email: user.email.clone(),
        };

        self.repo.save_user(user).await?;
        self.repo.save_event(event).await?;

        Ok(())
    }

    pub async fn handle_get_user_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<models::User>, sqlx::Error> {
        self.repo.find_user_by_id(id).await
    }
}
```
- Note: You are not limited to one repo to inject here

</details>


<details>
<summary>8. Create Route Handler Function (Controller)</summary>

<br>

```rust
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    commands,
    services::{self, UserService},
};

pub async fn create_user(
    State(handler): State<UserService>,
    Json(payload): Json<commands::CreateUser>,
) -> impl IntoResponse {
    match handler.handle_create_user(payload).await {
        Ok(_) => {
            info!("User Created");
            "User created".into_response()
        }
        Err(_) => {
            error!("Failed to Create User");
            "Failed to create user".into_response()
        }
    }
}
pub async fn get_user_by_id(
    State(state): State<services::UserService>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.handle_get_user_by_id(id).await {
        Ok(Some(user)) => {
            info!("User Found:\n {:#?}", user);
            Json(user).into_response()
        }
        Ok(None) => {
            info!("User Not Found");
            "User not found".into_response()
        }
        Err(_) => {
            error!("Failed to Fetch User");
            "Failed to get user".into_response()
        }
    }
}
```
- Note: You have access to State as first Parameter

</details>


---

### Testing APIs

#### Create new User 

1. Using Curl

```http
curl -X POST \
     -H "Content-Type: application/json" \
     -d '{"username": "testuser", "email": "testuser@example.com"}' \
     http://127.0.0.1:80/users
```


#### Get User by UUID

1. Using Curl
```http
curl localhost:80/users/01911459-8cfa-7e91-9f2a-4d3da4faa526
```
