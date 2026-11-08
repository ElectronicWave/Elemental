use std::collections::{HashMap, HashSet};

use elemental::infra::downloader::{
    DownloadEvent, DownloadProgress, SessionSnapshot, TaskId, TaskStatus,
};

#[derive(Debug, Default)]
pub(super) struct DownloadPhase {
    task_groups: HashMap<TaskId, Option<String>>,
    active_tasks: HashSet<TaskId>,
    active_groups: HashMap<String, usize>,
    last_group: Option<String>,
    progress: DownloadProgress,
}

impl DownloadPhase {
    pub(super) fn from_snapshot(snapshot: SessionSnapshot) -> Self {
        let mut phase = Self {
            progress: snapshot.progress,
            ..Self::default()
        };

        for task in snapshot.tasks {
            if task.group.is_some() {
                phase.last_group = task.group.clone();
            }
            phase
                .task_groups
                .insert(task.task_id.clone(), task.group.clone());
            if matches!(task.status, TaskStatus::Active) {
                phase.mark_active(task.task_id);
            }
        }

        phase
    }

    pub(super) fn progress(&self) -> &DownloadProgress {
        &self.progress
    }

    pub(super) fn apply(&mut self, event: DownloadEvent) -> bool {
        match event {
            DownloadEvent::TaskQueued { task, .. } => {
                if task.group.is_some() {
                    self.last_group = task.group.clone();
                }
                self.task_groups.insert(task.task_id, task.group);
                true
            }
            DownloadEvent::TaskStarted { task_id, .. } => {
                self.mark_active(task_id);
                true
            }
            DownloadEvent::TaskFinished { outcome, .. } => {
                self.task_groups
                    .insert(outcome.task_id.clone(), outcome.group);
                self.mark_finished(&outcome.task_id);
                true
            }
            DownloadEvent::SessionProgress { progress, .. } => {
                self.progress = progress;
                false
            }
            DownloadEvent::SessionOpened { .. }
            | DownloadEvent::TaskProgress { .. }
            | DownloadEvent::TaskRetrying { .. }
            | DownloadEvent::SessionClosed { .. } => false,
        }
    }

    pub(super) fn label(&self) -> Option<String> {
        if !self.active_groups.is_empty() {
            let mut groups: Vec<&str> = self.active_groups.keys().map(String::as_str).collect();
            groups.sort_unstable();
            return Some(groups.join(", "));
        }

        self.last_group.clone()
    }

    fn mark_active(&mut self, task_id: TaskId) {
        if !self.active_tasks.insert(task_id.clone()) {
            return;
        }

        let Some(Some(group)) = self.task_groups.get(&task_id) else {
            return;
        };
        *self.active_groups.entry(group.clone()).or_default() += 1;
    }

    fn mark_finished(&mut self, task_id: &TaskId) {
        if !self.active_tasks.remove(task_id) {
            return;
        }

        let Some(group) = self.task_groups.get(task_id).cloned().flatten() else {
            return;
        };
        let becomes_empty = match self.active_groups.get_mut(&group) {
            Some(count) => {
                *count = count.saturating_sub(1);
                *count == 0
            }
            None => false,
        };
        if becomes_empty {
            self.active_groups.remove(&group);
        }
    }
}
