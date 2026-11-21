# Grow entries
## Requirements
* Users must sign in and can only access their own entries and photos.
* An entry belongs to one grow log and has a date, stage, and optional description.
* A grow has at most one entry per date.
* Stages progress from germination to seedling, vegetation, and bloom. Stages may be skipped.
* Log details show entries newest first.
* A photo belongs to one entry. Each upload contains exactly one JPEG or PNG file and is limited to 10 MiB.
* Photos have originals and JPEG thumbnails no larger than 1024 by 1024 pixels.
* Deleting an entry or log also deletes its photos.

## User scenarios
### 1. User adds an entry to a grow log
Request:
```http
POST /logs/1/entries
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

```json
{
  "occurred_at": "2024-03-08T10:30:00Z",
  "stage": "seedling",
  "description": "First true leaves"
}
```

Response, `201 Created`:
```json
{
  "id": 2,
  "grow_log_id": 1,
  "occurred_at": "2024-03-08T10:30:00Z",
  "stage": "seedling",
  "description": "First true leaves",
  "inserted_at": "2024-03-08T10:30:00Z",
  "updated_at": "2024-03-08T10:30:00Z",
  "photos": []
}
```

### 2. User uploads a photo
Multipart request:
```http
POST /entries/2/photos
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
Content-Type: multipart/form-data; boundary=photo

--photo
Content-Disposition: form-data; name="file"; filename="seedling.jpg"
Content-Type: image/jpeg

<jpeg bytes>
--photo--
```

Response, `201 Created`:
```json
{
  "id": 4,
  "media_type": "image/jpeg",
  "original_filename": "seedling.jpg",
  "url": "/photos/4",
  "thumbnail_url": "/photos/4/thumbnail",
  "inserted_at": "2024-03-08T10:31:00Z",
  "updated_at": "2024-03-08T10:31:00Z"
}
```

### 3. User updates an entry
Request:
```http
PATCH /entries/2
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

```json
{
  "occurred_at": "2024-03-08T10:30:00Z",
  "stage": "vegetation",
  "description": "Transplanted"
}
```

Response, `200 OK`:
```json
{
  "id": 2,
  "grow_log_id": 1,
  "occurred_at": "2024-03-08T10:30:00Z",
  "stage": "vegetation",
  "description": "Transplanted",
  "inserted_at": "2024-03-08T10:30:00Z",
  "updated_at": "2024-03-09T09:00:00Z",
  "photos": [
    {
      "id": 4,
      "media_type": "image/jpeg",
      "original_filename": "seedling.jpg",
      "url": "/photos/4",
      "thumbnail_url": "/photos/4/thumbnail",
      "inserted_at": "2024-03-08T10:31:00Z",
      "updated_at": "2024-03-08T10:31:00Z"
    }
  ]
}
```

### 4. User cannot use an invalid stage
Request:
```http
POST /logs/1/entries
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

```json
{
  "occurred_at": "2024-03-08T10:30:00Z",
  "stage": "flowering"
}
```

Response, `422 Unprocessable Entity`:
```json
{
  "error": "validation failed",
  "fields": [
    {
      "field": "stage",
      "message": "must be germination, seedling, vegetation, or bloom"
    }
  ]
}
```

### 5. User cannot access another user's photo
Request:
```http
GET /photos/4
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoiYW5vdGhlckBleGFtcGxlLmNvbSIsImRldmljZV9pZCI6bnVsbCwiZGV2aWNlX25hbWUiOm51bGx9.D3r477qnfxD90REBgpQ1s6AMDa-JQ0H_z7TyYJ67fAY
```

Response, `404 Not Found`:
```json
{
  "error": "not found"
}
```

### 6. User reads uploaded photo and thumbnail
A user retrieves an uploaded JPEG using the photo URLs returned by the upload request.

Request:
```http
GET /photos/4
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

Response, `200 OK`:
```http
Content-Type: image/jpeg
Cache-Control: private, no-store
```

The response body contains the uploaded JPEG bytes.

Request:
```http
GET /photos/4/thumbnail
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

Response, `200 OK`:
```http
Content-Type: image/jpeg
Cache-Control: private, no-store
```

The response body contains a JPEG thumbnail no larger than 1024 by 1024 pixels.

### 7. A user cannot create an entry on an occupied date
Mustard log `1` already has a vegetation entry on 15 April 2026.

Request:
```http
POST /logs/1/entries
Cookie: jwt=<jwt>
```

```json
{
  "occurred_at": "2026-04-15T12:00:00Z",
  "stage": "vegetation"
}
```

Response, `422 Unprocessable Entity`:
```json
{
  "error": "validation failed",
  "fields": [
    {
      "field": "occurred_at",
      "message": "an entry already exists for this date"
    }
  ]
}
```

### 8. A user cannot move an entry to an occupied date
Mustard log `1` has vegetation entries on 1 and 15 April 2026. Entry `3` is the 15 April entry.

Request:
```http
PATCH /entries/3
Cookie: jwt=<jwt>
```

```json
{
  "occurred_at": "2026-04-01T12:00:00Z",
  "stage": "vegetation"
}
```

Response, `422 Unprocessable Entity`:
```json
{
  "error": "validation failed",
  "fields": [
    {
      "field": "occurred_at",
      "message": "an entry already exists for this date"
    }
  ]
}
```

### 9. A user cannot record an earlier stage
Mustard log `1` entered vegetation on 1 April 2026 and bloom on 1 May.

Request:
```http
POST /logs/1/entries
Cookie: jwt=<jwt>
```

```json
{
  "occurred_at": "2026-05-06T10:30:00Z",
  "stage": "vegetation"
}
```

Response, `422 Unprocessable Entity`:
```json
{
  "error": "validation failed",
  "fields": [
    {
      "field": "stage",
      "message": "must follow the order of the other entries"
    }
  ]
}
```

### 10. A user cannot change an earlier entry to a later stage
Mustard log `1` has vegetation entries on 1 and 15 April 2026. Entry `2` is the 1 April entry.

Request:
```http
PATCH /entries/2
Cookie: jwt=<jwt>
```

```json
{
  "occurred_at": "2026-04-01T10:30:00Z",
  "stage": "bloom"
}
```

Response, `422 Unprocessable Entity`:
```json
{
  "error": "validation failed",
  "fields": [
    {
      "field": "stage",
      "message": "must follow the order of the other entries"
    }
  ]
}
```

### 11. A user skips a stage
Mustard log `1` has a germination entry on 20 March 2026.

Request:
```http
POST /logs/1/entries
Cookie: jwt=<jwt>
```

```json
{
  "occurred_at": "2026-04-01T10:30:00Z",
  "stage": "vegetation"
}
```

Response, `201 Created`:
```json
{
  "id": 2,
  "grow_log_id": 1,
  "occurred_at": "2026-04-01T10:30:00Z",
  "stage": "vegetation",
  "description": null,
  "inserted_at": "2026-04-01T10:30:00Z",
  "updated_at": "2026-04-01T10:30:00Z",
  "photos": []
}
```
