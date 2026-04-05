-- EmberFlow SQLite schema v1
-- Canonical runtime store for tracks, tasks, events, projections, and
-- canonical track metadata/brief/plan state.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS tracks (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  track_type TEXT,
  status TEXT NOT NULL CHECK (
    status IN ('planning', 'in-progress', 'blocked', 'review', 'done', 'archived')
  ),
  description TEXT,
  branch TEXT,
  spec_ref TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS track_brief_sections (
  id TEXT PRIMARY KEY,
  track_id TEXT NOT NULL,
  section_key TEXT NOT NULL,
  section_text TEXT NOT NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(track_id, section_key),
  UNIQUE(track_id, position),
  FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS track_plan_phases (
  id TEXT PRIMARY KEY,
  track_id TEXT NOT NULL,
  phase_key TEXT NOT NULL,
  title TEXT NOT NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(track_id, phase_key),
  UNIQUE(track_id, position),
  FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS track_plan_items (
  id TEXT PRIMARY KEY,
  track_id TEXT NOT NULL,
  phase_id TEXT NOT NULL,
  title TEXT NOT NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(phase_id, position),
  FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
  FOREIGN KEY (phase_id) REFERENCES track_plan_phases(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  track_id TEXT,
  plan_item_id TEXT,
  title TEXT NOT NULL,
  status TEXT NOT NULL CHECK (
    status IN ('queued', 'running', 'need-input', 'blocked', 'awaiting-review', 'done', 'failed', 'cancelled')
  ),
  phase TEXT NOT NULL CHECK (
    phase IN ('exploring', 'planning', 'implementing', 'verifying')
  ),
  executor TEXT,
  agent_instance_id TEXT,
  execution TEXT,
  intent_summary TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE SET NULL,
  FOREIGN KEY (plan_item_id) REFERENCES track_plan_items(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS events (
  id TEXT PRIMARY KEY,
  track_id TEXT,
  task_id TEXT,
  kind TEXT NOT NULL CHECK (
    kind IN ('assign', 'ack', 'progress', 'blocker', 'handoff', 'close', 'claim', 'release', 'lease-expired')
  ),
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE SET NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS projections (
  id TEXT PRIMARY KEY,
  event_id TEXT NOT NULL,
  projection_kind TEXT NOT NULL CHECK (
    projection_kind IN ('user', 'runtime', 'track')
  ),
  target_path TEXT,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(event_id, projection_kind),
  FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS projection_dirty_targets (
  id TEXT PRIMARY KEY,
  track_id TEXT,
  projection_kind TEXT NOT NULL,
  target_path TEXT NOT NULL UNIQUE,
  reason TEXT NOT NULL,
  source_event_id TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE SET NULL,
  FOREIGN KEY (source_event_id) REFERENCES events(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_tasks_track_id ON tasks(track_id);
CREATE INDEX IF NOT EXISTS idx_tasks_plan_item_id ON tasks(plan_item_id);
CREATE INDEX IF NOT EXISTS idx_events_track_id ON events(track_id);
CREATE INDEX IF NOT EXISTS idx_events_task_id ON events(task_id);
CREATE INDEX IF NOT EXISTS idx_track_brief_sections_track_id ON track_brief_sections(track_id);
CREATE INDEX IF NOT EXISTS idx_track_plan_phases_track_id ON track_plan_phases(track_id);
CREATE INDEX IF NOT EXISTS idx_track_plan_items_track_id ON track_plan_items(track_id);
CREATE INDEX IF NOT EXISTS idx_track_plan_items_phase_id ON track_plan_items(phase_id);
CREATE INDEX IF NOT EXISTS idx_projections_event_id ON projections(event_id);
CREATE INDEX IF NOT EXISTS idx_projection_dirty_targets_track_id ON projection_dirty_targets(track_id);
