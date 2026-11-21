# Grow logs
## Requirements
* Users must sign in to manage their grow logs.
* A grow log belongs to one of the user's plants and requires a name and start date.
* A grow log may have a description, end date, and outcome. An end date cannot precede the start date. A log without an end date is active.
* Grow logs are ordered by their latest entry, newest first. Logs without entries appear last.
* A grow log's cover is its newest photo, even when its latest entry has no photo.
* Users can only access their own grow logs.
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
