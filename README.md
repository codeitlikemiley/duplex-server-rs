CQRS + Event Sourcing on Axum


## Create new User 
```http
curl -X POST \
     -H "Content-Type: application/json" \
     -d '{"username": "testuser", "email": "testuser@example.com"}' \
     http://127.0.0.1:3000/users
```

## Get User by UUID
```http
curl localhost:3000/users/01911459-8cfa-7e91-9f2a-4d3da4faa526
```

---
1. Create Routes Api 

2. Create Command with Deserialize

3. Create Events with Serialize and Deserialize

4. Create Models with Serde Traits and sqlx::FromRow

5. Create Repository with method signatures no Impl

- Note: you might need to use `axum::async_trait` for async fn
- Note: We need to pass in either a Model or an Event here

6. Create Impl Repository on Struct with DbPool Connection e.g. `PgPool`
Note: if this dont exist you need to create `db.rs`

7. Create a Service that uses one or more repository

8. Create Controllers 
e.g. user_controller.rs
that has functions related to user 
it can have the state injected
and have access to payload and path params

here we can use the repo method 
