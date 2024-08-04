## DDD + CQRS + EventSourcing 

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


### Workflow Grpc

<details>
<summary>1. Create Protobuf</summary>
<br>

Note: It is important to note that any Command / Querries are equivalent to RPC

While both Request and Response can be used by Either Rest or Grpc Service

Through the build.rs we made all Request and Response Be `De/Serializable` 

```proto
syntax = "proto3";
package users;

// we can define here all our commands and querries
// as rpc
// while request and response for the messages
service UserService {
    rpc CreateUser(CreateUserRequest) returns (CreateUserResponse);

    rpc GetUser(GetUserRequest) returns (GetUserResponse);
}

message CreateUserRequest {
    string username =1;
    string email = 2;
}

message CreateUserResponse {}

message GetUserRequest {
    string id = 1;
}

message GetUserResponse {
    string id = 1;
    string username = 2;
    string email = 3;
}

```

</details>


<details>
<summary>2. Impl From Trait on a Command</summary>

<br>

Note: Here we made use of the generated `Message` on proto like `CreateUserRequest` to a command `CreateUser`

```rust
impl From<CreateUserRequest> for CreateUser {
    fn from(value: CreateUserRequest) -> Self {
        CreateUser {
            email: value.email,
            username: value.username,
        }
    }
}
```

<br>

Note: This will help us on Grpc Impl to just use `CreateUser::from(request.into_inner())`

And converting any request to command that we can use our our service provider

</details>



<details>
<summary>3. Create/Add Grpc Service</summary>
<br>

Note: It is important that you passed a state of db pool connection to a `service_provider`


```rust
use sqlx::{Pool, Postgres};
use tonic_reflection::pb::v1alpha::FILE_DESCRIPTOR_SET;

use super::users::GrpcUserServiceImpl;

pub fn services(pool: Pool<Postgres>) -> axum::routing::Router {
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    tonic::transport::Server::builder()
        .accept_http1(true)
        .add_service(reflection_service)
        .add_service(GrpcUserServiceImpl::new(pool.clone()))
        /// Add your new Service here
        .into_router()
}

```

Note: you can enable `tonic_web` on specific service by doing this

```rust        
.add_service(tonic_web::enable(GrpcUserServiceImpl::new(pool.clone())))
```


</details>


<details>
<summary>4. Implement Rpc</summary>

<br>

```rust
use sqlx::{Pool, Postgres};
use tonic::{Request, Response, Status};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    commands::CreateUser,
    proto::{
        user_service_server::{UserService as GrpcUserService, UserServiceServer},
        CreateUserRequest, CreateUserResponse, GetUserRequest, GetUserResponse,
    },
    services::UserService,
    PostgreSQL,
};

#[derive(Debug)]
pub struct GrpcUserServiceImpl {
    repo: UserService,
}

impl GrpcUserServiceImpl {
    pub fn new(pool: Pool<Postgres>) -> UserServiceServer<GrpcUserServiceImpl> {
        let user_service = UserService::new(PostgreSQL::new(pool.clone()));
        UserServiceServer::new(GrpcUserServiceImpl { repo: user_service })
    }
}

#[tonic::async_trait]
impl GrpcUserService for GrpcUserServiceImpl {
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<CreateUserResponse>, Status> {
        let command = CreateUser::from(request.into_inner());

        match self.repo.handle_create_user(command).await {
            Ok(_) => {
                info!("User Created");
                Ok(Response::new(CreateUserResponse {}))
            }
            Err(e) => {
                error!("{}", e);
                Err(Status::already_exists("User already Exists"))
            }
        }
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<GetUserResponse>, Status> {
        let id = Uuid::parse_str(&request.into_inner().id).unwrap();

        match self.repo.handle_get_user_by_id(id).await {
            Ok(Some(user)) => {
                info!("User Found:\n{:#?}", user);
                let response = Response::new(GetUserResponse {
                    id: user.id.to_string(),
                    email: user.email,
                    username: user.username,
                });

                Ok(response)
            }
            Ok(None) => Err(Status::not_found("User Not Found")),
            Err(e) => {
                error!("{}", e);
                Err(Status::not_found("User Not Found"))
            }
        }
    }
}

```

</details>


---

### Testing APIs

#### Create new User 

1. Using GrpCurl

```http
curl -X POST \
     -H "Content-Type: application/json" \
     -d '{"username": "testuser", "email": "testuser@example.com"}' \
     http://127.0.0.1:80/users
```

2. Using Postman
- Create new GRPC
- Enter Url: `grpc://localhost:80`
- Import `users.proto`
- Add the Payload on `Message`

```json
{
    "username": "uriah",
    "email": "ceo@goldcoders.dev"
}
```



#### Get User by UUID

1. Using GrpCurl
```http
curl localhost:80/users/01911459-8cfa-7e91-9f2a-4d3da4faa526
```

2. Using Postman
- Create new GRPC
- Enter Url: `grpc://localhost:80`
- Import `users.proto`
- Add the Payload on `Message`

```json
{
    "id": "01911459-8cfa-7e91-9f2a-4d3da4faa526",
}
```


