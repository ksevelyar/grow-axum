# Users
## Requirements
* Users can create an account without signing in.
* Email, handle, password, and time zone are required.
* Email addresses and handles are unique. Email uniqueness is case-insensitive.
* The time zone must be an IANA time zone name.
* Users' passwords are not disclosed.

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
