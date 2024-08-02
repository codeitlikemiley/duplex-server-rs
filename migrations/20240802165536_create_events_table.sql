-- Add migration script here
CREATE TABLE events (
    id UUID PRIMARY KEY,
    payload JSONB NOT NULL
)
