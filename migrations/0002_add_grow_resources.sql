CREATE TABLE plants (
  id BIGSERIAL PRIMARY KEY,
  user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  name TEXT NOT NULL CHECK (length(trim(name)) > 0),
  inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (user_id, name),
  UNIQUE (user_id, id)
);

CREATE TABLE grow_logs (
  id BIGSERIAL PRIMARY KEY,
  user_id BIGINT NOT NULL,
  plant_id BIGINT NOT NULL,
  name TEXT NOT NULL CHECK (length(trim(name)) > 0),
  started_at TIMESTAMPTZ NOT NULL,
  description TEXT,
  ended_at TIMESTAMPTZ,
  outcome TEXT,
  inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  FOREIGN KEY (user_id, plant_id) REFERENCES plants(user_id, id) ON DELETE RESTRICT,
  CHECK (ended_at IS NULL OR ended_at >= started_at)
);

CREATE INDEX grow_logs_user_id_idx ON grow_logs(user_id);
CREATE INDEX grow_logs_plant_id_idx ON grow_logs(plant_id);

CREATE TABLE grow_log_entries (
  id BIGSERIAL PRIMARY KEY,
  grow_log_id BIGINT NOT NULL REFERENCES grow_logs(id) ON DELETE CASCADE,
  occurred_at TIMESTAMPTZ NOT NULL,
  stage TEXT NOT NULL CHECK (stage IN ('germination', 'seedling', 'vegetation', 'bloom')),
  description TEXT,
  inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX grow_log_entries_log_occurred_at_idx
  ON grow_log_entries(grow_log_id, occurred_at DESC, id DESC);

CREATE TABLE grow_log_entry_photos (
  id BIGSERIAL PRIMARY KEY,
  grow_log_entry_id BIGINT NOT NULL REFERENCES grow_log_entries(id) ON DELETE CASCADE,
  storage_key TEXT NOT NULL UNIQUE,
  media_type TEXT NOT NULL,
  original_filename TEXT,
  inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX grow_log_entry_photos_entry_id_idx ON grow_log_entry_photos(grow_log_entry_id);
