//! Snapshot-based undo/redo for project edits.
//!
//! Projects are small enough to clone, so the editor records the project state
//! before an edit instead of maintaining inverse commands. Rapid edits with the
//! same key (typing a name, dragging a corner or slider) are coalesced into one
//! undo step.

use std::time::{Duration, Instant};

use lpmt_core::model::Project;

const MAX_HISTORY: usize = 100;
const COALESCE_WINDOW: Duration = Duration::from_millis(800);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HistoryKey {
    SurfaceCorner(usize, usize),
    SurfaceGridPoint(usize, u16, u16),
    SurfaceGeometry,
    SurfaceName(usize),
    SurfaceOpacity(usize),
    SurfaceMaskPoint(usize, usize),
    SurfaceMaskFeather(usize),
    MediaPath(usize),
    LayerName(usize),
    LayerOpacity(usize),
    LayerCrop(usize),
    ShowAudioVolume,
    OutputView(usize),
    OutputEdgeBlend(usize),
    OutputColor(usize),
    Discrete,
}

impl HistoryKey {
    fn coalesces(self) -> bool {
        self != Self::Discrete
    }
}

struct HistoryEntry {
    project: Project,
    label: &'static str,
}

pub(crate) struct History {
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    last_key: Option<HistoryKey>,
    last_recorded: Instant,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            last_key: None,
            last_recorded: Instant::now(),
        }
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.last_key = None;
    }

    /// End a continuous edit so the next edit starts a new undo step.
    pub fn break_coalescing(&mut self) {
        self.last_key = None;
    }

    /// Record the project state from before an edit. A no-op edit must not call
    /// this method.
    pub fn record(&mut self, before: Project, key: HistoryKey, label: &'static str) {
        let coalesce = key.coalesces()
            && self.last_key == Some(key)
            && self.last_recorded.elapsed() <= COALESCE_WINDOW;
        if !coalesce {
            self.undo.push(HistoryEntry {
                project: before,
                label,
            });
            if self.undo.len() > MAX_HISTORY {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
        self.last_key = if key.coalesces() { Some(key) } else { None };
        self.last_recorded = Instant::now();
    }

    /// Restore the previous project state. `current` is stored for redo.
    pub fn undo(&mut self, current: Project) -> Option<(Project, &'static str)> {
        self.last_key = None;
        while let Some(entry) = self.undo.pop() {
            if entry.project == current {
                continue;
            }
            let label = entry.label;
            self.redo.push(HistoryEntry {
                project: current,
                label,
            });
            return Some((entry.project, label));
        }
        None
    }

    /// Restore the next project state after an undo.
    pub fn redo(&mut self, current: Project) -> Option<(Project, &'static str)> {
        self.last_key = None;
        while let Some(entry) = self.redo.pop() {
            if entry.project == current {
                continue;
            }
            let label = entry.label;
            self.undo.push(HistoryEntry {
                project: current,
                label,
            });
            return Some((entry.project, label));
        }
        None
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_label(&self) -> Option<&'static str> {
        self.undo.last().map(|entry| entry.label)
    }

    pub fn redo_label(&self) -> Option<&'static str> {
        self.redo.last().map(|entry| entry.label)
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn renamed(project: &Project, name: &str) -> Project {
        let mut project = project.clone();
        project.name = name.to_owned();
        project
    }

    #[test]
    fn undo_and_redo_restore_project_states() {
        let mut history = History::new();
        let p0 = Project::demo();
        let p1 = renamed(&p0, "one");
        let p2 = renamed(&p1, "two");

        history.record(p0.clone(), HistoryKey::Discrete, "rename");
        history.record(p1.clone(), HistoryKey::Discrete, "rename");

        let (restored, label) = history.undo(p2.clone()).expect("undo");
        assert_eq!(restored, p1);
        assert_eq!(label, "rename");

        let (restored, _) = history.redo(p1).expect("redo");
        assert_eq!(restored, p2);
    }

    #[test]
    fn rapid_edits_with_the_same_key_are_coalesced() {
        let mut history = History::new();
        let p0 = Project::demo();
        let p1 = renamed(&p0, "one");
        let p2 = renamed(&p1, "two");

        history.record(p0.clone(), HistoryKey::SurfaceOpacity(0), "change opacity");
        history.record(p1, HistoryKey::SurfaceOpacity(0), "change opacity");

        let (restored, _) = history.undo(p2).expect("undo");
        assert_eq!(restored, p0);
    }

    #[test]
    fn different_keys_are_separate_undo_steps() {
        let mut history = History::new();
        let p0 = Project::demo();
        let p1 = renamed(&p0, "one");
        let p2 = renamed(&p1, "two");

        history.record(p0, HistoryKey::SurfaceCorner(0, 0), "move corner");
        history.record(p1.clone(), HistoryKey::SurfaceCorner(0, 1), "move corner");

        let (restored, _) = history.undo(p2).expect("undo");
        assert_eq!(restored, p1);
    }

    #[test]
    fn breaking_coalescing_starts_a_new_undo_step() {
        let mut history = History::new();
        let p0 = Project::demo();
        let p1 = renamed(&p0, "one");
        let p2 = renamed(&p1, "two");

        history.record(p0, HistoryKey::SurfaceOpacity(0), "change opacity");
        history.break_coalescing();
        history.record(p1.clone(), HistoryKey::SurfaceOpacity(0), "change opacity");

        let (restored, _) = history.undo(p2).expect("undo");
        assert_eq!(restored, p1);
    }

    #[test]
    fn a_new_edit_clears_redo() {
        let mut history = History::new();
        let p0 = Project::demo();
        let p1 = renamed(&p0, "one");
        let p2 = renamed(&p1, "two");

        history.record(p0.clone(), HistoryKey::Discrete, "rename");
        assert!(history.undo(p1.clone()).is_some());
        assert!(history.can_redo());

        history.record(p0.clone(), HistoryKey::Discrete, "rename");
        assert!(!history.can_redo());
        assert_eq!(history.undo_label(), Some("rename"));
        assert_eq!(history.redo_label(), None);

        // The new edit's before-state is the state restored by the undo.
        let (restored, _) = history.undo(p2).expect("undo");
        assert_eq!(restored, p0);
    }
}
