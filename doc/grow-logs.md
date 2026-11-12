# Grow logs
## Requirements
* All log endpoints require an authenticated session.
* A grow log belongs to one owned plant. Its name and `started_at` are required.
* `started_at` and `ended_at` are RFC 3339 timestamps. An end cannot precede its start.
* Logs include optional `description`, `ended_at`, and `outcome`. A missing end means the log is active.
* The `/logs` list is ordered by latest entry `occurred_at`, newest first. Logs without entries follow logs with entries. Log id descending breaks ties.
* A log summary's thumbnail points to the newest photo across its entries, skipping entries without photos.
* Resources belonging to another user return `404 Not Found`.
* Deleting a log also deletes its entries and photos.
## User scenarios
### 1. User creates a grow log
Request:
```http
POST /logs
Cookie: jwt=<jwt>
```

```json
{
  "plant_id": 1,
  "name": "Spring grow",
  "started_at": "2024-03-01T09:00:00Z",
  "description": "Started indoors",
  "ended_at": null,
  "outcome": null
}
```

Response, `201 Created`:
```json
{
  "id": 1,
  "plant_id": 1,
  "name": "Spring grow",
  "started_at": "2024-03-01T09:00:00Z",
  "description": "Started indoors",
  "ended_at": null,
  "outcome": null,
  "inserted_at": "2024-03-01T09:00:00Z",
  "updated_at": "2024-03-01T09:00:00Z",
  "entries": []
}
```

### 2. User sees grow logs ordered by latest entry
Setup:
The user has a Mustard log with an entry today, a Clover log with an entry yesterday, and a Sunflower log with no entries.

Request:
```http
GET /logs
Cookie: jwt=<jwt>
```

Response, `200 OK`, ordered by latest entry and then id descending:
```json
[
  {
    "id": 3,
    "plant_id": 1,
    "name": "Mustard",
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
    "name": "Clover",
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
    "name": "Sunflower",
    "started_at": "2024-03-03T00:00:00Z",
    "description": null,
    "ended_at": null,
    "outcome": null,
    "thumbnail_url": null,
    "inserted_at": "2024-03-03T00:00:00Z",
    "updated_at": "2024-03-03T00:00:00Z"
  }
]
```

### 3. User completes a grow log
Request:
```http
PATCH /logs/1
Cookie: jwt=<jwt>
```

```json
{
  "plant_id": 1,
  "name": "Spring grow",
  "started_at": "2024-03-01T09:00:00Z",
  "description": "Started indoors",
  "ended_at": "2024-07-01T12:00:00Z",
  "outcome": "Harvested"
}
```

Response, `200 OK`:
```json
{
  "id": 1,
  "plant_id": 1,
  "name": "Spring grow",
  "started_at": "2024-03-01T09:00:00Z",
  "description": "Started indoors",
  "ended_at": "2024-07-01T12:00:00Z",
  "outcome": "Harvested",
  "inserted_at": "2024-03-01T09:00:00Z",
  "updated_at": "2024-07-01T12:00:00Z",
  "entries": []
}
```
