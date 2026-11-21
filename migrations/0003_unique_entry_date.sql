CREATE UNIQUE INDEX grow_log_entries_log_date_unique
  ON grow_log_entries (grow_log_id, ((occurred_at AT TIME ZONE 'UTC')::date));
