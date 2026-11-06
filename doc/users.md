# Users
## Registration
`POST /users` creates an account.

Request body:
```json
{
  "email": "person@example.com",
  "handle": "person",
  "password": "correct horse battery staple",
  "timezone": "Europe/London"
}
```

`timezone` must be an IANA timezone name. A successful request returns `201 Created`:
```json
{
  "id": 1,
  "email": "person@example.com",
  "timezone": "Europe/London"
}
```

An already registered email or invalid timezone returns `422 Unprocessable Entity` with field errors.

## Sessions
`POST /sessions` authenticates an email and password.

Request body:
```json
{
  "email": "person@example.com",
  "password": "correct horse battery staple"
}
```

A successful request returns `201 Created` and sets a `jwt` cookie. The cookie is HTTP-only, scoped to `/`, lasts one year, and is secure outside debug builds. Invalid credentials return `401 Unauthorized`.

`GET /sessions/current` returns the authenticated user. It requires the `jwt` cookie and returns `401 Unauthorized` when the cookie is missing, invalid, expired, or belongs to a deleted user.

## Configuration
`JWT_SECRET` signs and verifies session tokens. It must be set before session endpoints are used.
