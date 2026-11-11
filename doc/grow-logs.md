# Grow logs
## Requirements
* All endpoints require an authenticated session.
* Plants belong to one user. Names are required, trimmed, and unique per user.
* A plant detail includes its logs. Plant and log indexes are ordered newest first by their respective timestamps.
* A grow log belongs to one owned plant. Its name and `started_at` are required.
* `started_at` and `ended_at` are RFC 3339 timestamps. An end cannot precede its start.
* Logs include optional `description`, `ended_at`, and `outcome`. A missing end means the log is active.
* A log summary's thumbnail points to the newest photo across its entries, skipping entries without photos.
* Deleting a plant referenced by a log returns `409 Conflict`.
* Resources belonging to another user return `404 Not Found`.
* Deleting a log also deletes its entries and photos.

## User scenarios
### 1. Create a plant
Request:
```http
POST /plants
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
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

### 2. Create a grow log
Request:
```http
POST /logs
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
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

### 3. List grow logs
Request:
```http
GET /logs
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

Response, `200 OK`:
```json
[
  {
    "id": 1,
    "plant_id": 1,
    "name": "Spring grow",
    "started_at": "2024-03-01T09:00:00Z",
    "description": "Started indoors",
    "ended_at": null,
    "outcome": null,
    "thumbnail_url": "/photos/4/thumbnail",
    "inserted_at": "2024-03-01T09:00:00Z",
    "updated_at": "2024-03-01T09:00:00Z"
  }
]
```

### 4. Complete a grow log
Request:
```http
PATCH /logs/1
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
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

### 5. Reject deletion of a referenced plant
Request:
```http
DELETE /plants/1
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

Response, `409 Conflict`:
```json
{
  "error": "conflict"
}
```
