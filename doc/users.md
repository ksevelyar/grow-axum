# Users
## Requirements
* `POST /users` creates an account without an authenticated session.
* `email`, `handle`, `password`, and `timezone` are required.
* Email addresses and handles are unique. Email uniqueness is case-insensitive.
* `timezone` must be an IANA timezone name.
* Responses never include the password or its hash.

## User scenarios
### 1. User registers an account
Request:
```http
POST /users
```

```json
{
  "email": "person@example.com",
  "handle": "person",
  "password": "correct horse battery staple",
  "timezone": "Europe/London"
}
```

Response, `201 Created`:
```json
{
  "id": 1,
  "email": "person@example.com",
  "timezone": "Europe/London"
}
```

### 2. User cannot register an account with an email already in use
Request:
```http
POST /users
```

```json
{
  "email": "PERSON@example.com",
  "handle": "another-person",
  "password": "correct horse battery staple",
  "timezone": "Europe/London"
}
```

Response, `422 Unprocessable Entity`:
```json
{
  "error": "validation failed",
  "fields": [
    {
      "field": "email",
      "message": "email is already taken"
    }
  ]
}
```

### 3. User cannot register an account with an unknown timezone
Request:
```http
POST /users
```

```json
{
  "email": "person@example.com",
  "handle": "person",
  "password": "correct horse battery staple",
  "timezone": "Mars/Olympus_Mons"
}
```

Response, `422 Unprocessable Entity`:
```json
{
  "error": "validation failed",
  "fields": [
    {
      "field": "timezone",
      "message": "unknown timezone"
    }
  ]
}
```
