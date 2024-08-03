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
