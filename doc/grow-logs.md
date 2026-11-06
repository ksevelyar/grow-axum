# Grow logs
## Scope
Authenticated users manage their own plants, grow logs, log entries, and entry photos. A user cannot read or modify another user's resources.

## Plants
A plant belongs to one user and has an id and name.

* A plant name is required and unique within its owner's plants.
* Users can create, list, update, and delete their plants.
* Viewing a plant shows all grow logs for that plant.
* Deleting a plant that is referenced by one or more grow logs returns `409 Conflict`.

## Grow logs
A grow log belongs to one user and references exactly one of that user's plants.

* A grow log has an id, plant id, required name, required UTC start timestamp, optional description, optional UTC end timestamp, and optional text outcome.
* A user can create, list, view, update, and delete their grow logs.
* The selected plant must belong to the authenticated user.
* An end timestamp cannot be earlier than the start timestamp.
* A log without an end timestamp is active. A log with an end timestamp is completed.
* Bloom light scheduling is not part of this version.

## Log entries
A log entry belongs to one grow log.

* An entry has an id, required UTC date timestamp, required stage, optional description, and zero or more photos.
* Valid stages are `germination`, `seedling`, `vegetation`, and `bloom`.
* A user can create, update, and delete entries for their grow logs.
* A user cannot access entries through a grow log owned by another user.
* Entries are displayed newest first.
* Entry photos are optional.

## Photos
Photos belong to one log entry and are stored on the server filesystem.

* A user can upload multiple photos for an entry.
* The server generates photo identifiers and storage names. It must not use a client-provided filename as a storage path.
* The server serves photos only to the owner of the containing grow log.
* Deleting an entry deletes its photo files. Deleting a grow log deletes its entries and their photo files.

## API
All endpoints require an authenticated session.

### Plants
* `GET /plants` lists the authenticated user's plants.
* `POST /plants` creates a plant.
* `GET /plants/{plant_id}` returns one owned plant and its grow logs.
* `PATCH /plants/{plant_id}` updates one owned plant.
* `DELETE /plants/{plant_id}` deletes one owned unreferenced plant.

### Logs
* `GET /logs` lists the authenticated user's active and completed logs.
* `POST /logs` creates a grow log.
* `GET /logs/{log_id}` returns one owned grow log and all its entries, ordered newest first.
* `PATCH /logs/{log_id}` updates one owned grow log.
* `DELETE /logs/{log_id}` deletes one owned grow log.

The log list includes the most recent entry photo when one exists. The client uses it as the gallery thumbnail.

### Entries
* `POST /logs/{log_id}/entries` creates an entry for an owned grow log.
* `PATCH /entries/{entry_id}` updates one owned entry.
* `DELETE /entries/{entry_id}` deletes one owned entry and its photos.

### Entry photos
* `POST /entries/{entry_id}/photos` uploads one photo using `multipart/form-data`.
* `GET /photos/{photo_id}` returns one owned photo.
* `DELETE /photos/{photo_id}` deletes one owned photo.

## Errors
* Missing or invalid sessions return `401 Unauthorized`.
* A resource outside the authenticated user's ownership returns `404 Not Found`.
* Invalid request fields return `422 Unprocessable Entity` with field errors.
* Deleting a referenced plant returns `409 Conflict`.
