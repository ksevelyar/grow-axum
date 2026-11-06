# Grow entries
## Requirements
* All endpoints require an authenticated session and only expose the authenticated user's resources.
* An entry belongs to one grow log and has required RFC 3339 `occurred_at` and `stage` fields.
* Valid stages are `germination`, `seedling`, `vegetation`, and `bloom`.
* `description` is optional. Log details order entries newest first by `occurred_at`, then id.
* A photo belongs to one entry. Each upload contains exactly one JPEG or PNG file and is limited to 10 MiB.
* The server generates the storage key and returns original and thumbnail URLs. Thumbnails are JPEG images no larger than 1024 by 1024 pixels.
* Photo reads use `Cache-Control: private, no-store`.
* Deleting an entry or log deletes its photo records, originals, and thumbnails.

## User scenarios
### 1. Add an entry to a grow log
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

### 2. Upload a photo
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

### 3. Update an entry
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

### 4. Reject an invalid stage
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

### 5. Reject access to another user's photo
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
