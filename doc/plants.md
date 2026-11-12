# Plants
## Requirements
* All plant endpoints require an authenticated session.
* Plants belong to one user. Names are required, trimmed, and unique per user.
* The `/plants` list is ordered by name and id.
* A plant detail includes its grow logs ordered by latest entry `occurred_at`, newest first. Logs without entries follow logs with entries. Log id descending breaks ties.
* Deleting a plant referenced by a log returns `409 Conflict`.
* Resources belonging to another user return `404 Not Found`.
## User scenarios
### 1. User creates a plant
Request:
```http
POST /plants
Cookie: jwt=<jwt>
```

```json
{
  "name": "Tomato"
}
```

Response, `201 Created`:
```json
{
  "id": 1,
  "name": "Tomato",
  "inserted_at": "2024-01-01T00:00:00Z",
  "updated_at": "2024-01-01T00:00:00Z"
}
```

### 2. User sees a plant's grow logs ordered by latest entry
Setup:
The user has a Mustard plant with a Mustard 0 log with an entry today, a Mustard 1 log with an entry yesterday, and a Mustard 4L log with no entries.

Request:
```http
GET /plants/1
Cookie: jwt=<jwt>
```

Response, `200 OK`, with `grow_logs` ordered by latest entry and then id descending:
```json
{
  "id": 1,
  "name": "Mustard",
  "inserted_at": "2024-03-01T00:00:00Z",
  "updated_at": "2024-03-01T00:00:00Z",
  "grow_logs": [
    {
      "id": 3,
      "plant_id": 1,
      "name": "Mustard 0",
      "started_at": "2024-03-01T00:00:00Z",
      "description": null,
      "ended_at": null,
      "outcome": null,
      "thumbnail_url": null,
      "inserted_at": "2024-03-01T00:00:00Z",
      "updated_at": "2024-03-01T00:00:00Z"
    },
    {
      "id": 2,
      "plant_id": 1,
      "name": "Mustard 1",
      "started_at": "2024-03-02T00:00:00Z",
      "description": null,
      "ended_at": null,
      "outcome": null,
      "thumbnail_url": null,
      "inserted_at": "2024-03-02T00:00:00Z",
      "updated_at": "2024-03-02T00:00:00Z"
    },
    {
      "id": 1,
      "plant_id": 1,
      "name": "Mustard 4L",
      "started_at": "2024-03-03T00:00:00Z",
      "description": null,
      "ended_at": null,
      "outcome": null,
      "thumbnail_url": null,
      "inserted_at": "2024-03-03T00:00:00Z",
      "updated_at": "2024-03-03T00:00:00Z"
    }
  ]
}
```

### 3. User cannot delete a plant with grow logs
Request:
```http
DELETE /plants/1
Cookie: jwt=<jwt>
```

Response, `409 Conflict`:
```json
{
  "error": "conflict"
}
```
