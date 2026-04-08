use crate::error::{EmberFlowError, Result};
use crate::protocol::{validate_choice, PHASES, RUNTIME_MESSAGES, TASK_STATUSES, TRACK_STATUSES};
use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackRecord {
    pub id: String,
    pub title: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackMetadataInput {
    pub track_id: String,
    pub track_type: String,
    pub status: String,
    pub description: String,
    pub branch: String,
    pub spec_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackMetadataRecord {
    pub track_id: String,
    pub track_type: String,
    pub status: String,
    pub description: String,
    pub branch: String,
    pub spec_ref: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackBriefSectionInput {
    pub section_key: String,
    pub section_text: String,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackBriefSectionRecord {
    pub track_id: String,
    pub section_key: String,
    pub section_text: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackBriefRecord {
    pub track_id: String,
    pub sections: Vec<TrackBriefSectionRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackPlanItemInput {
    pub item_id: String,
    pub title: String,
    pub position: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackPlanItemRecord {
    pub item_id: String,
    pub track_id: String,
    pub phase_id: String,
    pub title: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackPlanPhaseInput {
    pub phase_id: String,
    pub title: String,
    pub position: i64,
    pub items: Vec<TrackPlanItemInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackPlanPhaseRecord {
    pub phase_id: String,
    pub track_id: String,
    pub title: String,
    pub position: i64,
    pub items: Vec<TrackPlanItemRecord>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackPlanRecord {
    pub track_id: String,
    pub phases: Vec<TrackPlanPhaseRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRecord {
    pub id: String,
    pub track_id: Option<String>,
    pub plan_item_id: Option<String>,
    pub title: String,
    pub status: String,
    pub phase: String,
    pub executor: Option<String>,
    pub agent_instance_id: Option<String>,
    pub execution: Option<String>,
    pub intent_summary: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub lease_holder: Option<String>,
    pub lease_expires_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskInput {
    pub task_id: String,
    pub track_id: Option<String>,
    pub title: String,
    pub status: String,
    pub phase: String,
    pub executor: Option<String>,
    pub agent_instance_id: Option<String>,
    pub execution: Option<String>,
    pub intent_summary: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TaskStateUpdate<'a> {
    pub status: Option<&'a str>,
    pub phase: Option<&'a str>,
    pub track_id: Option<&'a str>,
    pub executor: Option<&'a str>,
    pub agent_instance_id: Option<&'a str>,
    pub execution: Option<&'a str>,
    pub intent_summary: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRecord {
    pub id: String,
    pub track_id: Option<String>,
    pub task_id: Option<String>,
    pub kind: String,
    pub payload: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionRecord {
    pub id: String,
    pub event_id: String,
    pub projection_kind: String,
    pub target_path: Option<String>,
    pub payload: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionDirtyTargetInput {
    pub track_id: Option<String>,
    pub projection_kind: String,
    pub target_path: String,
    pub reason: String,
    pub source_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionDirtyTargetRecord {
    pub id: String,
    pub track_id: Option<String>,
    pub projection_kind: String,
    pub target_path: String,
    pub reason: String,
    pub source_event_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseInfo {
    pub holder: String,
    pub acquired_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeStore {
    db_path: PathBuf,
}

impl RuntimeStore {
    fn normalize_track_status(status: &str) -> &str {
        match status {
            "done" => "archived",
            other => other,
        }
    }

    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let store = Self { db_path };
        store.initialize()?;
        Ok(store)
    }

    fn initialize(&self) -> Result<()> {
        let schema = include_str!("../../runtime/sqlite/schema.sql");
        let conn = self.connect()?;
        conn.execute_batch(schema)?;
        self.ensure_task_visibility_columns(&conn)?;
        self.ensure_task_lease_columns(&conn)?;
        Ok(())
    }

    fn ensure_task_visibility_columns(&self, conn: &Connection) -> Result<()> {
        self.add_column_if_missing(conn, "ALTER TABLE tasks ADD COLUMN executor TEXT")?;
        self.add_column_if_missing(conn, "ALTER TABLE tasks ADD COLUMN execution TEXT")?;
        self.add_column_if_missing(conn, "ALTER TABLE tasks ADD COLUMN intent_summary TEXT")?;
        if self.table_has_column(conn, "tasks", "agent_type")? {
            conn.execute(
                "UPDATE tasks SET executor = COALESCE(executor, agent_type) WHERE executor IS NULL",
                [],
            )?;
        }
        Ok(())
    }

    fn ensure_task_lease_columns(&self, conn: &Connection) -> Result<()> {
        self.add_column_if_missing(conn, "ALTER TABLE tasks ADD COLUMN lease_holder TEXT")?;
        self.add_column_if_missing(conn, "ALTER TABLE tasks ADD COLUMN lease_acquired_at TEXT")?;
        self.add_column_if_missing(conn, "ALTER TABLE tasks ADD COLUMN lease_expires_at TEXT")?;
        Ok(())
    }

    fn add_column_if_missing(&self, conn: &Connection, statement: &str) -> Result<()> {
        match conn.execute(statement, []) {
            Ok(_) => Ok(()),
            Err(rusqlite::Error::SqliteFailure(_, Some(message)))
                if message.contains("duplicate column name") =>
            {
                Ok(())
            }
            Err(error) => Err(error.into()),
        }
    }

    fn table_has_column(&self, conn: &Connection, table: &str, column: &str) -> Result<bool> {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        for row in rows {
            if row? == column {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn connect(&self) -> Result<Connection> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 5000;",
        )?;
        Ok(conn)
    }

    fn to_payload(value: &Value) -> Result<String> {
        Ok(serde_json::to_string(value)?)
    }

    fn from_payload(value: &str) -> Result<Value> {
        Ok(serde_json::from_str(value)?)
    }

    fn get_track_optional(&self, track_id: &str) -> Result<Option<TrackRecord>> {
        let conn = self.connect()?;
        let track = conn
            .query_row(
                "SELECT id, title, status, created_at, updated_at FROM tracks WHERE id = ?",
                rusqlite::params![track_id],
                |row| {
                    Ok(TrackRecord {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        status: row.get(2)?,
                        created_at: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                },
            )
            .optional()?;
        Ok(track)
    }

    fn get_task_optional(&self, task_id: &str) -> Result<Option<TaskRecord>> {
        let conn = self.connect()?;
        let task = conn
            .query_row(
                "SELECT id, track_id, plan_item_id, title, status, phase, executor, agent_instance_id, execution, intent_summary, created_at, updated_at, lease_holder, lease_expires_at FROM tasks WHERE id = ?",
                rusqlite::params![task_id],
                |row| {
                    Ok(TaskRecord {
                        id: row.get(0)?,
                        track_id: row.get(1)?,
                        plan_item_id: row.get(2)?,
                        title: row.get(3)?,
                        status: row.get(4)?,
                        phase: row.get(5)?,
                        executor: row.get(6)?,
                        agent_instance_id: row.get(7)?,
                        execution: row.get(8)?,
                        intent_summary: row.get(9)?,
                        created_at: row.get(10)?,
                        updated_at: row.get(11)?,
                        lease_holder: row.get(12)?,
                        lease_expires_at: row.get(13)?,
                    })
                },
            )
            .optional()?;
        Ok(task)
    }

    fn get_plan_item_optional(&self, item_id: &str) -> Result<Option<TrackPlanItemRecord>> {
        let conn = self.connect()?;
        let item = conn
            .query_row(
                "SELECT id, track_id, phase_id, title, position, created_at, updated_at FROM track_plan_items WHERE id = ?",
                rusqlite::params![item_id],
                |row| {
                    Ok(TrackPlanItemRecord {
                        item_id: row.get(0)?,
                        track_id: row.get(1)?,
                        phase_id: row.get(2)?,
                        title: row.get(3)?,
                        position: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .optional()?;
        Ok(item)
    }

    pub(crate) fn get_track_metadata_optional(
        &self,
        track_id: &str,
    ) -> Result<Option<TrackMetadataRecord>> {
        let conn = self.connect()?;
        let metadata = conn
            .query_row(
                "SELECT id, track_type, status, description, branch, spec_ref, created_at, updated_at FROM tracks WHERE id = ?",
                rusqlite::params![track_id],
                |row| {
                    let track_type: Option<String> = row.get(1)?;
                    match track_type {
                        None => Ok(None),
                        Some(track_type) => Ok(Some(TrackMetadataRecord {
                            track_id: row.get(0)?,
                            track_type,
                            status: row.get(2)?,
                            description: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                            branch: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                            spec_ref: row.get(5)?,
                            created_at: row.get(6)?,
                            updated_at: row.get(7)?,
                        })),
                    }
                },
            )
            .optional()?;
        Ok(metadata.flatten())
    }

    pub fn list_tracks(&self) -> Result<Vec<TrackRecord>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, title, status, created_at, updated_at FROM tracks ORDER BY created_at ASC, id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(TrackRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn list_active_tracks(&self) -> Result<Vec<TrackRecord>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, title, status, created_at, updated_at FROM tracks WHERE status != 'archived' ORDER BY created_at ASC, id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(TrackRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn list_tasks(&self) -> Result<Vec<TaskRecord>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, track_id, plan_item_id, title, status, phase, executor, agent_instance_id, execution, intent_summary, created_at, updated_at, lease_holder, lease_expires_at FROM tasks ORDER BY created_at ASC, id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(TaskRecord {
                id: row.get(0)?,
                track_id: row.get(1)?,
                plan_item_id: row.get(2)?,
                title: row.get(3)?,
                status: row.get(4)?,
                phase: row.get(5)?,
                executor: row.get(6)?,
                agent_instance_id: row.get(7)?,
                execution: row.get(8)?,
                intent_summary: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
                lease_holder: row.get(12)?,
                lease_expires_at: row.get(13)?,
            })
        })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    fn get_track_brief_sections(&self, track_id: &str) -> Result<Vec<TrackBriefSectionRecord>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT track_id, section_key, section_text, position, created_at, updated_at FROM track_brief_sections WHERE track_id = ? ORDER BY position ASC, section_key ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![track_id], |row| {
            Ok(TrackBriefSectionRecord {
                track_id: row.get(0)?,
                section_key: row.get(1)?,
                section_text: row.get(2)?,
                position: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;

        let mut sections = Vec::new();
        for row in rows {
            sections.push(row?);
        }
        Ok(sections)
    }

    fn get_track_plan_phases(&self, track_id: &str) -> Result<Vec<TrackPlanPhaseRecord>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, track_id, phase_key, title, position, created_at, updated_at FROM track_plan_phases WHERE track_id = ? ORDER BY position ASC, phase_key ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![track_id], |row| {
            Ok(TrackPlanPhaseRecord {
                phase_id: row.get(2)?,
                track_id: row.get(1)?,
                title: row.get(3)?,
                position: row.get(4)?,
                items: Vec::new(),
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;

        let mut phases = Vec::new();
        for row in rows {
            phases.push(row?);
        }

        for phase in &mut phases {
            phase.items = self.get_track_plan_items(&phase.phase_id)?;
        }

        Ok(phases)
    }

    fn get_track_plan_items(&self, phase_id: &str) -> Result<Vec<TrackPlanItemRecord>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, track_id, phase_id, title, position, created_at, updated_at FROM track_plan_items WHERE phase_id = ? ORDER BY position ASC, id ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![phase_id], |row| {
            Ok(TrackPlanItemRecord {
                item_id: row.get(0)?,
                track_id: row.get(1)?,
                phase_id: row.get(2)?,
                title: row.get(3)?,
                position: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    fn get_event_optional(&self, event_id: &str) -> Result<Option<EventRecord>> {
        let conn = self.connect()?;
        let event = conn
            .query_row(
                "SELECT id, track_id, task_id, kind, payload_json, created_at FROM events WHERE id = ?",
                rusqlite::params![event_id],
                |row| {
                    let payload_json: String = row.get(4)?;
                    Ok(EventRecord {
                        id: row.get(0)?,
                        track_id: row.get(1)?,
                        task_id: row.get(2)?,
                        kind: row.get(3)?,
                        payload: Self::from_payload(&payload_json).map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                4,
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        })?,
                        created_at: row.get(5)?,
                    })
                },
            )
            .optional()?;
        Ok(event)
    }

    fn get_projection_optional(
        &self,
        event_id: &str,
        projection_kind: &str,
    ) -> Result<Option<ProjectionRecord>> {
        let conn = self.connect()?;
        let projection = conn
            .query_row(
                "SELECT id, event_id, projection_kind, target_path, payload_json, created_at FROM projections WHERE event_id = ? AND projection_kind = ?",
                rusqlite::params![event_id, projection_kind],
                |row| {
                    let payload_json: String = row.get(4)?;
                    Ok(ProjectionRecord {
                        id: row.get(0)?,
                        event_id: row.get(1)?,
                        projection_kind: row.get(2)?,
                        target_path: row.get(3)?,
                        payload: Self::from_payload(&payload_json).map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                4,
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        })?,
                        created_at: row.get(5)?,
                    })
                },
            )
            .optional()?;
        Ok(projection)
    }

    fn get_dirty_projection_target_optional(
        &self,
        target_path: &str,
    ) -> Result<Option<ProjectionDirtyTargetRecord>> {
        let conn = self.connect()?;
        let target = conn
            .query_row(
                "SELECT id, track_id, projection_kind, target_path, reason, source_event_id, created_at, updated_at FROM projection_dirty_targets WHERE target_path = ?",
                rusqlite::params![target_path],
                |row| {
                    Ok(ProjectionDirtyTargetRecord {
                        id: row.get(0)?,
                        track_id: row.get(1)?,
                        projection_kind: row.get(2)?,
                        target_path: row.get(3)?,
                        reason: row.get(4)?,
                        source_event_id: row.get(5)?,
                        created_at: row.get(6)?,
                        updated_at: row.get(7)?,
                    })
                },
            )
            .optional()?;
        Ok(target)
    }

    fn ensure_track_exists(&self, track_id: &str) -> Result<()> {
        if self.get_track_optional(track_id)?.is_none() {
            return Err(EmberFlowError::NotFound(track_id.to_string()));
        }
        Ok(())
    }

    fn ensure_task_exists(&self, task_id: &str) -> Result<()> {
        if self.get_task_optional(task_id)?.is_none() {
            return Err(EmberFlowError::NotFound(task_id.to_string()));
        }
        Ok(())
    }

    fn ensure_plan_item_exists(&self, item_id: &str) -> Result<TrackPlanItemRecord> {
        self.get_plan_item_optional(item_id)?
            .ok_or_else(|| EmberFlowError::NotFound(item_id.to_string()))
    }

    pub(crate) fn ensure_track(&self, track_id: &str) -> Result<TrackRecord> {
        match self.get_track(track_id) {
            Ok(track) => Ok(track),
            Err(_) => self.create_track(track_id, track_id, "planning"),
        }
    }

    pub(crate) fn ensure_task(&self, task_id: &str, track_id: &str) -> Result<TaskRecord> {
        match self.get_task(task_id) {
            Ok(task) => Ok(task),
            Err(_) => self.create_task(TaskInput {
                task_id: task_id.to_string(),
                track_id: Some(track_id.to_string()),
                title: task_id.to_string(),
                status: "queued".to_string(),
                phase: "planning".to_string(),
                executor: None,
                agent_instance_id: None,
                execution: None,
                intent_summary: None,
            }),
        }
    }

    pub fn create_track(&self, track_id: &str, title: &str, status: &str) -> Result<TrackRecord> {
        let status = Self::normalize_track_status(status);
        validate_choice(status, TRACK_STATUSES, "status")?;
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO tracks (id, title, status) VALUES (?, ?, ?)",
            rusqlite::params![track_id, title, status],
        )?;
        self.get_track(track_id)
    }

    pub fn upsert_track_metadata(&self, input: TrackMetadataInput) -> Result<TrackMetadataRecord> {
        let TrackMetadataInput {
            track_id,
            track_type,
            status,
            description,
            branch,
            spec_ref,
        } = input;
        let status = Self::normalize_track_status(&status).to_string();
        validate_choice(&status, TRACK_STATUSES, "status")?;
        if track_id.trim().is_empty() {
            return Err(EmberFlowError::UnsupportedValue {
                field: "track_id",
                value: "track id must be present".to_string(),
            });
        }
        let title = if description.trim().is_empty() {
            track_id.clone()
        } else {
            description.clone()
        };
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO tracks (id, title, status, track_type, description, branch, spec_ref) VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET title = excluded.title, status = excluded.status, track_type = excluded.track_type, description = excluded.description, branch = excluded.branch, spec_ref = excluded.spec_ref, updated_at = datetime('now')",
            rusqlite::params![
                track_id.clone(),
                title,
                status.clone(),
                track_type,
                description,
                branch,
                spec_ref
            ],
        )?;
        self.get_track_metadata(&track_id)
    }

    pub fn get_track_metadata(&self, track_id: &str) -> Result<TrackMetadataRecord> {
        self.get_track_metadata_optional(track_id)?
            .ok_or_else(|| EmberFlowError::NotFound(track_id.to_string()))
    }

    pub fn replace_track_brief(
        &self,
        track_id: &str,
        sections: Vec<TrackBriefSectionInput>,
    ) -> Result<TrackBriefRecord> {
        self.ensure_track_exists(track_id)?;
        for section in &sections {
            if section.section_key.trim().is_empty() {
                return Err(EmberFlowError::UnsupportedValue {
                    field: "section_key",
                    value: "section key must be present".to_string(),
                });
            }
        }

        let mut conn = self.connect()?;
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM track_brief_sections WHERE track_id = ?",
            rusqlite::params![track_id],
        )?;
        for section in &sections {
            tx.execute(
                "INSERT INTO track_brief_sections (id, track_id, section_key, section_text, position) VALUES (?, ?, ?, ?, ?)",
                rusqlite::params![
                    format!("{track_id}:brief:{}", section.section_key),
                    track_id,
                    section.section_key.as_str(),
                    section.section_text.as_str(),
                    section.position
                ],
            )?;
        }
        tx.commit()?;

        self.get_track_brief(track_id)
    }

    pub fn get_track_brief(&self, track_id: &str) -> Result<TrackBriefRecord> {
        self.ensure_track_exists(track_id)?;
        let sections = self.get_track_brief_sections(track_id)?;
        Ok(TrackBriefRecord {
            track_id: track_id.to_string(),
            sections,
        })
    }

    pub fn replace_track_plan(
        &self,
        track_id: &str,
        phases: Vec<TrackPlanPhaseInput>,
    ) -> Result<TrackPlanRecord> {
        self.ensure_track_exists(track_id)?;
        for phase in &phases {
            if phase.phase_id.trim().is_empty() {
                return Err(EmberFlowError::UnsupportedValue {
                    field: "phase_id",
                    value: "phase id must be present".to_string(),
                });
            }
            for item in &phase.items {
                if item.item_id.trim().is_empty() || item.position.is_none() {
                    return Err(EmberFlowError::UnsupportedValue {
                        field: "item",
                        value: "stable placement requires a phase item id and position".to_string(),
                    });
                }
            }
        }

        let mut conn = self.connect()?;
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM track_plan_phases WHERE track_id = ?",
            rusqlite::params![track_id],
        )?;
        for phase in &phases {
            tx.execute(
                "INSERT INTO track_plan_phases (id, track_id, phase_key, title, position) VALUES (?, ?, ?, ?, ?)",
                rusqlite::params![
                    phase.phase_id.as_str(),
                    track_id,
                    phase.phase_id.as_str(),
                    phase.title.as_str(),
                    phase.position
                ],
            )?;
            for item in &phase.items {
                let position = item.position.expect("validated above");
                tx.execute(
                    "INSERT INTO track_plan_items (id, track_id, phase_id, title, position) VALUES (?, ?, ?, ?, ?)",
                    rusqlite::params![
                        item.item_id.as_str(),
                        track_id,
                        phase.phase_id.as_str(),
                        item.title.as_str(),
                        position
                    ],
                )?;
            }
        }
        tx.commit()?;

        self.get_track_plan(track_id)
    }

    pub fn get_track_plan(&self, track_id: &str) -> Result<TrackPlanRecord> {
        self.ensure_track_exists(track_id)?;
        let phases = self.get_track_plan_phases(track_id)?;
        Ok(TrackPlanRecord {
            track_id: track_id.to_string(),
            phases,
        })
    }

    pub fn get_track(&self, track_id: &str) -> Result<TrackRecord> {
        self.get_track_optional(track_id)?
            .ok_or_else(|| EmberFlowError::NotFound(track_id.to_string()))
    }

    pub fn update_track_status(&self, track_id: &str, status: &str) -> Result<TrackRecord> {
        let status = Self::normalize_track_status(status);
        validate_choice(status, TRACK_STATUSES, "status")?;
        let conn = self.connect()?;
        let affected = conn.execute(
            "UPDATE tracks SET status = ?, updated_at = datetime('now') WHERE id = ?",
            rusqlite::params![status, track_id],
        )?;
        if affected == 0 {
            return Err(EmberFlowError::NotFound(track_id.to_string()));
        }
        self.get_track(track_id)
    }

    pub fn delete_track(&self, track_id: &str) -> Result<TrackRecord> {
        let track = self.get_track(track_id)?;
        let mut conn = self.connect()?;
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM projection_dirty_targets WHERE track_id = ? OR source_event_id IN (SELECT id FROM events WHERE track_id = ? OR task_id IN (SELECT id FROM tasks WHERE track_id = ? OR plan_item_id IN (SELECT id FROM track_plan_items WHERE track_id = ?)))",
            rusqlite::params![track_id, track_id, track_id, track_id],
        )?;
        tx.execute(
            "DELETE FROM events WHERE track_id = ? OR task_id IN (SELECT id FROM tasks WHERE track_id = ? OR plan_item_id IN (SELECT id FROM track_plan_items WHERE track_id = ?))",
            rusqlite::params![track_id, track_id, track_id],
        )?;
        tx.execute(
            "DELETE FROM tasks WHERE track_id = ? OR plan_item_id IN (SELECT id FROM track_plan_items WHERE track_id = ?)",
            rusqlite::params![track_id, track_id],
        )?;
        tx.execute(
            "DELETE FROM tracks WHERE id = ?",
            rusqlite::params![track_id],
        )?;
        tx.commit()?;
        Ok(track)
    }

    fn insert_task(&self, input: TaskInput, plan_item_id: Option<&str>) -> Result<TaskRecord> {
        let TaskInput {
            task_id,
            track_id,
            title,
            status,
            phase,
            executor,
            agent_instance_id,
            execution,
            intent_summary,
        } = input;
        validate_choice(&status, TASK_STATUSES, "status")?;
        validate_choice(&phase, PHASES, "phase")?;
        let plan_item = match plan_item_id {
            Some(plan_item_id) => Some(self.ensure_plan_item_exists(plan_item_id)?),
            None => None,
        };
        if let Some(track_id) = track_id.as_deref() {
            self.ensure_track_exists(track_id)?;
        }
        let resolved_track_id = match (track_id.clone(), plan_item.as_ref()) {
            (Some(track_id), Some(plan_item)) => {
                if plan_item.track_id != track_id {
                    return Err(EmberFlowError::UnsupportedValue {
                        field: "plan_item_id",
                        value: format!(
                            "track mismatch: plan item belongs to {} but task targets {}",
                            plan_item.track_id, track_id
                        ),
                    });
                }
                Some(track_id)
            }
            (Some(track_id), None) => Some(track_id),
            (None, Some(plan_item)) => Some(plan_item.track_id.clone()),
            (None, None) => None,
        };
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO tasks (id, track_id, plan_item_id, title, status, phase, executor, agent_instance_id, execution, intent_summary) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                task_id.clone(),
                resolved_track_id,
                plan_item_id,
                title,
                status,
                phase,
                executor,
                agent_instance_id,
                execution,
                intent_summary
            ],
        )?;
        self.get_task(&task_id)
    }

    pub fn create_task(&self, input: TaskInput) -> Result<TaskRecord> {
        self.insert_task(input, None)
    }

    pub fn create_task_for_plan_item(
        &self,
        input: TaskInput,
        plan_item_id: &str,
    ) -> Result<TaskRecord> {
        self.insert_task(input, Some(plan_item_id))
    }

    pub fn get_task(&self, task_id: &str) -> Result<TaskRecord> {
        self.get_task_optional(task_id)?
            .ok_or_else(|| EmberFlowError::NotFound(task_id.to_string()))
    }

    pub fn get_latest_task_for_track(&self, track_id: &str) -> Result<Option<TaskRecord>> {
        let conn = self.connect()?;
        let task = conn
            .query_row(
                "SELECT id, track_id, plan_item_id, title, status, phase, executor, agent_instance_id, execution, intent_summary, created_at, updated_at, lease_holder, lease_expires_at FROM tasks WHERE track_id = ? ORDER BY updated_at DESC, created_at DESC, id DESC LIMIT 1",
                rusqlite::params![track_id],
                |row| {
                    Ok(TaskRecord {
                        id: row.get(0)?,
                        track_id: row.get(1)?,
                        plan_item_id: row.get(2)?,
                        title: row.get(3)?,
                        status: row.get(4)?,
                        phase: row.get(5)?,
                        executor: row.get(6)?,
                        agent_instance_id: row.get(7)?,
                        execution: row.get(8)?,
                        intent_summary: row.get(9)?,
                        created_at: row.get(10)?,
                        updated_at: row.get(11)?,
                        lease_holder: row.get(12)?,
                        lease_expires_at: row.get(13)?,
                    })
                },
            )
            .optional()?;
        Ok(task)
    }

    pub fn update_task_state(
        &self,
        task_id: &str,
        update: TaskStateUpdate<'_>,
    ) -> Result<TaskRecord> {
        let current = self.get_task(task_id)?;
        let new_status = update.status.unwrap_or(&current.status);
        let new_phase = update.phase.unwrap_or(&current.phase);
        validate_choice(new_status, TASK_STATUSES, "status")?;
        validate_choice(new_phase, PHASES, "phase")?;
        if let Some(track_id) = update.track_id {
            self.ensure_track_exists(track_id)?;
        }
        let conn = self.connect()?;
        let affected = conn.execute(
            "UPDATE tasks SET track_id = ?, status = ?, phase = ?, executor = ?, agent_instance_id = ?, execution = ?, intent_summary = ?, updated_at = datetime('now') WHERE id = ?",
            rusqlite::params![
                update.track_id.or(current.track_id.as_deref()),
                new_status,
                new_phase,
                update.executor.or(current.executor.as_deref()),
                update
                    .agent_instance_id
                    .or(current.agent_instance_id.as_deref()),
                update.execution.or(current.execution.as_deref()),
                update
                    .intent_summary
                    .or(current.intent_summary.as_deref()),
                task_id
            ],
        )?;
        if affected == 0 {
            return Err(EmberFlowError::NotFound(task_id.to_string()));
        }
        self.get_task(task_id)
    }

    pub fn record_event(
        &self,
        event_id: &str,
        track_id: Option<&str>,
        task_id: Option<&str>,
        kind: &str,
        payload: Value,
    ) -> Result<EventRecord> {
        validate_choice(kind, RUNTIME_MESSAGES, "kind")?;
        if let Some(track_id) = track_id {
            self.ensure_track_exists(track_id)?;
        }
        if let Some(task_id) = task_id {
            self.ensure_task_exists(task_id)?;
        }
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO events (id, track_id, task_id, kind, payload_json) VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![
                event_id,
                track_id,
                task_id,
                kind,
                Self::to_payload(&payload)?
            ],
        )?;
        self.get_event(event_id)
    }

    pub fn get_event(&self, event_id: &str) -> Result<EventRecord> {
        self.get_event_optional(event_id)?
            .ok_or_else(|| EmberFlowError::NotFound(event_id.to_string()))
    }

    pub fn list_events(
        &self,
        track_id: Option<&str>,
        task_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<EventRecord>> {
        let mut query = String::from(
            "SELECT id, track_id, task_id, kind, payload_json, created_at FROM events",
        );
        let mut clauses = Vec::new();
        let mut values: Vec<rusqlite::types::Value> = Vec::new();
        if let Some(track_id) = track_id {
            clauses.push("track_id = ?");
            values.push(rusqlite::types::Value::Text(track_id.to_string()));
        }
        if let Some(task_id) = task_id {
            clauses.push("task_id = ?");
            values.push(rusqlite::types::Value::Text(task_id.to_string()));
        }
        if !clauses.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&clauses.join(" AND "));
        }
        query.push_str(" ORDER BY created_at ASC, id ASC");
        if let Some(limit) = limit {
            query.push_str(" LIMIT ?");
            values.push((limit as i64).into());
        }

        let conn = self.connect()?;
        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(values), |row| {
            let payload_json: String = row.get(4)?;
            Ok(EventRecord {
                id: row.get(0)?,
                track_id: row.get(1)?,
                task_id: row.get(2)?,
                kind: row.get(3)?,
                payload: Self::from_payload(&payload_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?,
                created_at: row.get(5)?,
            })
        })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn record_projection(
        &self,
        event_id: &str,
        projection_kind: &str,
        target_path: Option<&str>,
        payload: Value,
    ) -> Result<ProjectionRecord> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO projections (id, event_id, projection_kind, target_path, payload_json) VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![
                format!("{event_id}:{projection_kind}"),
                event_id,
                projection_kind,
                target_path,
                Self::to_payload(&payload)?
            ],
        )?;
        self.get_projection(event_id, projection_kind)
    }

    pub fn get_projection(
        &self,
        event_id: &str,
        projection_kind: &str,
    ) -> Result<ProjectionRecord> {
        self.get_projection_optional(event_id, projection_kind)?
            .ok_or_else(|| EmberFlowError::NotFound(format!("{event_id}:{projection_kind}")))
    }

    pub fn list_projections(
        &self,
        event_id: Option<&str>,
        projection_kind: Option<&str>,
    ) -> Result<Vec<ProjectionRecord>> {
        let mut query = String::from(
            "SELECT id, event_id, projection_kind, target_path, payload_json, created_at FROM projections",
        );
        let mut clauses = Vec::new();
        let mut values: Vec<rusqlite::types::Value> = Vec::new();
        if let Some(event_id) = event_id {
            clauses.push("event_id = ?");
            values.push(rusqlite::types::Value::Text(event_id.to_string()));
        }
        if let Some(projection_kind) = projection_kind {
            clauses.push("projection_kind = ?");
            values.push(rusqlite::types::Value::Text(projection_kind.to_string()));
        }
        if !clauses.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&clauses.join(" AND "));
        }
        query.push_str(" ORDER BY created_at ASC, id ASC");

        let conn = self.connect()?;
        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(values), |row| {
            let payload_json: String = row.get(4)?;
            Ok(ProjectionRecord {
                id: row.get(0)?,
                event_id: row.get(1)?,
                projection_kind: row.get(2)?,
                target_path: row.get(3)?,
                payload: Self::from_payload(&payload_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?,
                created_at: row.get(5)?,
            })
        })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn record_dirty_projection_target(
        &self,
        input: ProjectionDirtyTargetInput,
    ) -> Result<ProjectionDirtyTargetRecord> {
        let ProjectionDirtyTargetInput {
            track_id,
            projection_kind,
            target_path,
            reason,
            source_event_id,
        } = input;
        let id = target_path.clone();
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO projection_dirty_targets (id, track_id, projection_kind, target_path, reason, source_event_id) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(target_path) DO UPDATE SET track_id = excluded.track_id, projection_kind = excluded.projection_kind, reason = excluded.reason, source_event_id = excluded.source_event_id, updated_at = datetime('now')",
            rusqlite::params![
                id,
                track_id,
                projection_kind,
                target_path.clone(),
                reason,
                source_event_id
            ],
        )?;
        self.get_dirty_projection_target_optional(&target_path)?
            .ok_or_else(|| EmberFlowError::NotFound(target_path.clone()))
    }

    pub fn list_dirty_projection_targets(
        &self,
        track_id: Option<&str>,
    ) -> Result<Vec<ProjectionDirtyTargetRecord>> {
        let mut query = String::from(
            "SELECT id, track_id, projection_kind, target_path, reason, source_event_id, created_at, updated_at FROM projection_dirty_targets",
        );
        let mut values: Vec<rusqlite::types::Value> = Vec::new();
        if let Some(track_id) = track_id {
            query.push_str(" WHERE track_id = ?");
            values.push(rusqlite::types::Value::Text(track_id.to_string()));
        }
        query.push_str(" ORDER BY created_at ASC, id ASC");

        let conn = self.connect()?;
        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(values), |row| {
            Ok(ProjectionDirtyTargetRecord {
                id: row.get(0)?,
                track_id: row.get(1)?,
                projection_kind: row.get(2)?,
                target_path: row.get(3)?,
                reason: row.get(4)?,
                source_event_id: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn clear_dirty_projection_target(&self, target_path: &str) -> Result<bool> {
        let conn = self.connect()?;
        let affected = conn.execute(
            "DELETE FROM projection_dirty_targets WHERE target_path = ?",
            rusqlite::params![target_path],
        )?;
        Ok(affected > 0)
    }

    pub fn get_latest_event_for_track(&self, track_id: &str) -> Result<Option<EventRecord>> {
        let conn = self.connect()?;
        let event = conn
            .query_row(
                "SELECT id, track_id, task_id, kind, payload_json, created_at FROM events WHERE track_id = ? ORDER BY created_at DESC, id DESC LIMIT 1",
                rusqlite::params![track_id],
                |row| {
                    let payload_json: String = row.get(4)?;
                    Ok(EventRecord {
                        id: row.get(0)?,
                        track_id: row.get(1)?,
                        task_id: row.get(2)?,
                        kind: row.get(3)?,
                        payload: Self::from_payload(&payload_json).map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                4,
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        })?,
                        created_at: row.get(5)?,
                    })
                },
            )
            .optional()?;
        Ok(event)
    }

    pub fn get_latest_projection_for_track(
        &self,
        track_id: &str,
        projection_kind: &str,
    ) -> Result<Option<ProjectionRecord>> {
        let conn = self.connect()?;
        let projection = conn
            .query_row(
                "SELECT p.id, p.event_id, p.projection_kind, p.target_path, p.payload_json, p.created_at FROM projections AS p INNER JOIN events AS e ON e.id = p.event_id WHERE e.track_id = ? AND p.projection_kind = ? ORDER BY p.created_at DESC, p.id DESC LIMIT 1",
                rusqlite::params![track_id, projection_kind],
                |row| {
                    let payload_json: String = row.get(4)?;
                    Ok(ProjectionRecord {
                        id: row.get(0)?,
                        event_id: row.get(1)?,
                        projection_kind: row.get(2)?,
                        target_path: row.get(3)?,
                        payload: Self::from_payload(&payload_json).map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                4,
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        })?,
                        created_at: row.get(5)?,
                    })
                },
            )
            .optional()?;
        Ok(projection)
    }

    /// Claim exclusive lease on a task. If the same holder already holds the lease, it is
    /// refreshed. Returns an error if a different holder has an active (non-expired) lease.
    pub fn claim_task(
        &self,
        task_id: &str,
        holder: &str,
        duration_secs: Option<u64>,
    ) -> Result<LeaseInfo> {
        self.ensure_task_exists(task_id)?;
        let conn = self.connect()?;

        // Check current lease, lazily expiring if needed
        let existing = self.load_lease_from_conn(&conn, task_id)?;
        if let Some(ref lease) = existing {
            if lease.holder != holder {
                return Err(EmberFlowError::UnsupportedValue {
                    field: "lease_holder",
                    value: format!("task {} is already held by {}", task_id, lease.holder),
                });
            }
        }

        let expires_at = duration_secs.map(|secs| format!("datetime('now', '+{secs} seconds')",));

        match expires_at {
            Some(ref expr) => {
                conn.execute(
                    &format!(
                        "UPDATE tasks SET lease_holder = ?, lease_acquired_at = datetime('now'), lease_expires_at = {expr} WHERE id = ?"
                    ),
                    rusqlite::params![holder, task_id],
                )?;
            }
            None => {
                conn.execute(
                    "UPDATE tasks SET lease_holder = ?, lease_acquired_at = datetime('now'), lease_expires_at = NULL WHERE id = ?",
                    rusqlite::params![holder, task_id],
                )?;
            }
        }

        self.load_lease_from_conn(&conn, task_id)?
            .ok_or_else(|| EmberFlowError::NotFound(task_id.to_string()))
    }

    /// Release a lease held by `holder`. Returns error if holder doesn't match.
    pub fn release_task(&self, task_id: &str, holder: &str) -> Result<()> {
        self.ensure_task_exists(task_id)?;
        let conn = self.connect()?;

        let existing = self.load_lease_from_conn(&conn, task_id)?;
        match existing {
            None => Ok(()),
            Some(lease) if lease.holder == holder => {
                conn.execute(
                    "UPDATE tasks SET lease_holder = NULL, lease_acquired_at = NULL, lease_expires_at = NULL WHERE id = ?",
                    rusqlite::params![task_id],
                )?;
                Ok(())
            }
            Some(lease) => Err(EmberFlowError::UnsupportedValue {
                field: "lease_holder",
                value: format!(
                    "{holder} is not the lease holder for task {task_id} (held by {})",
                    lease.holder
                ),
            }),
        }
    }

    /// Return the current active lease for `task_id`, or None if no active lease.
    /// If the lease is expired, it is lazily cleared and None is returned.
    pub fn check_lease(&self, task_id: &str) -> Result<Option<LeaseInfo>> {
        let conn = self.connect()?;
        self.load_lease_from_conn(&conn, task_id)
    }

    /// Bulk-expire all tasks whose lease_expires_at is in the past. Returns count cleared.
    pub fn expire_stale_leases(&self) -> Result<u64> {
        let conn = self.connect()?;
        let affected = conn.execute(
            "UPDATE tasks SET lease_holder = NULL, lease_acquired_at = NULL, lease_expires_at = NULL WHERE lease_expires_at IS NOT NULL AND lease_expires_at <= datetime('now')",
            [],
        )?;
        Ok(affected as u64)
    }

    /// Claim a task with a fixed (possibly past) expires_at timestamp. Used in tests.
    #[doc(hidden)]
    pub fn claim_task_with_expiry(
        &self,
        task_id: &str,
        holder: &str,
        expires_at: &str,
    ) -> Result<()> {
        self.ensure_task_exists(task_id)?;
        let conn = self.connect()?;
        conn.execute(
            "UPDATE tasks SET lease_holder = ?, lease_acquired_at = datetime('now'), lease_expires_at = ? WHERE id = ?",
            rusqlite::params![holder, expires_at, task_id],
        )?;
        Ok(())
    }

    /// Read the raw lease columns for a task, lazily clearing if expired.
    fn load_lease_from_conn(&self, conn: &Connection, task_id: &str) -> Result<Option<LeaseInfo>> {
        let row = conn
            .query_row(
                "SELECT lease_holder, lease_acquired_at, lease_expires_at FROM tasks WHERE id = ?",
                rusqlite::params![task_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()?;

        let (holder, acquired_at, expires_at) = match row {
            None => return Ok(None),
            Some(tuple) => tuple,
        };

        let holder = match holder {
            None => return Ok(None),
            Some(h) => h,
        };
        let acquired_at = acquired_at.unwrap_or_default();

        // Lazy expiration: if expired, clear and return None
        if let Some(ref exp) = expires_at {
            let expired: bool = conn
                .query_row(
                    "SELECT ? <= datetime('now')",
                    rusqlite::params![exp],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            if expired {
                conn.execute(
                    "UPDATE tasks SET lease_holder = NULL, lease_acquired_at = NULL, lease_expires_at = NULL WHERE id = ?",
                    rusqlite::params![task_id],
                )?;
                return Ok(None);
            }
        }

        Ok(Some(LeaseInfo {
            holder,
            acquired_at,
            expires_at,
        }))
    }
}
