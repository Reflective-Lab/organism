use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Priority levels for tasks (beads visualization)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    /// Low priority - can wait (gray bead)
    Low = 1,
    /// Normal priority - standard tasks (blue bead)
    Normal = 2,
    /// High priority - should do soon (yellow bead)
    High = 3,
    /// Urgent - time-sensitive, do immediately (red bead)
    Urgent = 4,
    /// Critical - blocking other work (flashing red bead)
    Critical = 5,
}

impl Priority {
    /// Get the display color for TUI
    pub fn color(&self) -> &'static str {
        match self {
            Priority::Low => "gray",
            Priority::Normal => "blue",
            Priority::High => "yellow",
            Priority::Urgent => "red",
            Priority::Critical => "magenta",
        }
    }

    /// Get the bead symbol for TUI display
    pub fn bead(&self) -> &'static str {
        match self {
            Priority::Low => "○",      // empty circle
            Priority::Normal => "●",   // filled circle
            Priority::High => "◉",     // circle with dot
            Priority::Urgent => "◈",   // diamond in square
            Priority::Critical => "◆", // filled diamond
        }
    }

    /// Get human-readable label
    pub fn label(&self) -> &'static str {
        match self {
            Priority::Low => "Low",
            Priority::Normal => "Normal",
            Priority::High => "High",
            Priority::Urgent => "Urgent",
            Priority::Critical => "Critical",
        }
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// Current status of a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is waiting to be started
    Pending,
    /// Task is currently being worked on
    InProgress,
    /// Task completed successfully
    Completed,
    /// Task failed (with retries possible)
    Failed,
    /// Task was cancelled
    Cancelled,
    /// Task is blocked by another task
    Blocked,
}

impl TaskStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "⏳",
            TaskStatus::InProgress => "▶",
            TaskStatus::Completed => "✓",
            TaskStatus::Failed => "✗",
            TaskStatus::Cancelled => "⊘",
            TaskStatus::Blocked => "⊟",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Cancelled)
    }
}

impl Default for TaskStatus {
    fn default() -> Self {
        TaskStatus::Pending
    }
}

/// A task to fill a form
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,

    /// Reference to the form config to use
    pub form_config_id: Uuid,
    /// Reference to the profile to use
    pub profile_id: Uuid,

    pub priority: Priority,
    pub status: TaskStatus,

    /// When this task should be executed (for scheduling)
    pub scheduled_for: Option<DateTime<Utc>>,
    /// Deadline - task becomes urgent as this approaches
    pub deadline: Option<DateTime<Utc>>,

    /// Number of times this task has been attempted
    pub attempts: u32,
    /// Maximum retry attempts
    pub max_attempts: u32,

    /// Last error message if failed
    pub last_error: Option<String>,

    /// Tasks that must complete before this one
    pub depends_on: Vec<Uuid>,

    /// When the task was created
    pub created_at: DateTime<Utc>,
    /// When the task was last updated
    pub updated_at: DateTime<Utc>,
    /// When the task was completed
    pub completed_at: Option<DateTime<Utc>>,

    /// Optional tags for organization
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Task {
    pub fn new(name: String, form_config_id: Uuid, profile_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description: None,
            form_config_id,
            profile_id,
            priority: Priority::default(),
            status: TaskStatus::default(),
            scheduled_for: None,
            deadline: None,
            attempts: 0,
            max_attempts: 3,
            last_error: None,
            depends_on: Vec::new(),
            created_at: now,
            updated_at: now,
            completed_at: None,
            tags: Vec::new(),
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    pub fn with_scheduled_for(mut self, scheduled_for: DateTime<Utc>) -> Self {
        self.scheduled_for = Some(scheduled_for);
        self
    }

    pub fn with_depends_on(mut self, task_ids: Vec<Uuid>) -> Self {
        self.depends_on = task_ids;
        self
    }

    /// Check if task is ready to run (not blocked by dependencies)
    pub fn is_ready(&self, completed_tasks: &[Uuid]) -> bool {
        if self.status != TaskStatus::Pending {
            return false;
        }

        // Check if scheduled time has passed
        if let Some(scheduled) = self.scheduled_for {
            if scheduled > Utc::now() {
                return false;
            }
        }

        // Check dependencies
        self.depends_on.iter().all(|dep| completed_tasks.contains(dep))
    }

    /// Calculate effective priority based on deadline proximity
    pub fn effective_priority(&self) -> Priority {
        if let Some(deadline) = self.deadline {
            let now = Utc::now();
            let time_remaining = deadline.signed_duration_since(now);

            // Escalate priority as deadline approaches
            if time_remaining.num_hours() < 1 {
                return Priority::Critical;
            } else if time_remaining.num_hours() < 4 {
                return Priority::Urgent.max(self.priority);
            } else if time_remaining.num_hours() < 24 {
                return Priority::High.max(self.priority);
            }
        }

        self.priority
    }

    /// Mark task as in progress
    pub fn start(&mut self) {
        self.status = TaskStatus::InProgress;
        self.attempts += 1;
        self.updated_at = Utc::now();
    }

    /// Mark task as completed
    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Mark task as failed
    pub fn fail(&mut self, error: String) {
        self.last_error = Some(error);
        self.updated_at = Utc::now();

        if self.attempts >= self.max_attempts {
            self.status = TaskStatus::Failed;
        } else {
            self.status = TaskStatus::Pending; // Allow retry
        }
    }

    /// Mark task as cancelled
    pub fn cancel(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.updated_at = Utc::now();
    }
}

/// A queue of tasks sorted by priority
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskQueue {
    pub tasks: Vec<Task>,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn add(&mut self, task: Task) {
        self.tasks.push(task);
    }

    pub fn remove(&mut self, task_id: Uuid) -> Option<Task> {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == task_id) {
            Some(self.tasks.remove(pos))
        } else {
            None
        }
    }

    pub fn get(&self, task_id: Uuid) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == task_id)
    }

    pub fn get_mut(&mut self, task_id: Uuid) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == task_id)
    }

    /// Get completed task IDs
    pub fn completed_ids(&self) -> Vec<Uuid> {
        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .map(|t| t.id)
            .collect()
    }

    /// Get the next task to run (highest effective priority that is ready)
    pub fn next_ready(&self) -> Option<&Task> {
        let completed = self.completed_ids();

        self.tasks
            .iter()
            .filter(|t| t.is_ready(&completed))
            .max_by_key(|t| t.effective_priority())
    }

    /// Get all tasks sorted by effective priority (highest first)
    pub fn sorted_by_priority(&self) -> Vec<&Task> {
        let mut tasks: Vec<_> = self.tasks.iter().collect();
        tasks.sort_by(|a, b| b.effective_priority().cmp(&a.effective_priority()));
        tasks
    }

    /// Get tasks by status
    pub fn by_status(&self, status: TaskStatus) -> Vec<&Task> {
        self.tasks.iter().filter(|t| t.status == status).collect()
    }

    /// Get tasks by tag
    pub fn by_tag(&self, tag: &str) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|t| t.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// Summary counts for display
    pub fn summary(&self) -> TaskSummary {
        let mut summary = TaskSummary::default();

        for task in &self.tasks {
            match task.status {
                TaskStatus::Pending => summary.pending += 1,
                TaskStatus::InProgress => summary.in_progress += 1,
                TaskStatus::Completed => summary.completed += 1,
                TaskStatus::Failed => summary.failed += 1,
                TaskStatus::Cancelled => summary.cancelled += 1,
                TaskStatus::Blocked => summary.blocked += 1,
            }

            match task.effective_priority() {
                Priority::Critical => summary.critical += 1,
                Priority::Urgent => summary.urgent += 1,
                Priority::High => summary.high += 1,
                _ => {}
            }
        }

        summary.total = self.tasks.len();
        summary
    }
}

#[derive(Debug, Clone, Default)]
pub struct TaskSummary {
    pub total: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub blocked: usize,
    pub critical: usize,
    pub urgent: usize,
    pub high: usize,
}

impl std::fmt::Display for TaskSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tasks: {} total ({} pending, {} in progress, {} completed, {} failed)",
            self.total, self.pending, self.in_progress, self.completed, self.failed
        )?;

        if self.critical > 0 || self.urgent > 0 {
            write!(f, " [{}◆ {}◈]", self.critical, self.urgent)?;
        }

        Ok(())
    }
}
