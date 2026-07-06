use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Task {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub is_project: bool,
    pub is_payable: bool,
    pub is_archived: bool,
    pub total_duration_secs: u64,
}

#[derive(Debug)]
pub struct TaskTree {
    tasks: HashMap<i64, Task>,
    next_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeError {
    NotFound(i64),
    CyclicParent,
    InvalidParent,
    DuplicateName,
}

impl TaskTree {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn add_task(
        &mut self,
        parent_id: Option<i64>,
        name: &str,
        is_project: bool,
        is_payable: bool,
    ) -> Result<i64, TreeError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(TreeError::DuplicateName);
        }

        if self
            .tasks
            .values()
            .any(|t| t.name == name && t.parent_id == parent_id)
        {
            return Err(TreeError::DuplicateName);
        }

        if let Some(pid) = parent_id {
            if !self.tasks.contains_key(&pid) {
                return Err(TreeError::InvalidParent);
            }
            if self.would_create_cycle(pid, self.next_id) {
                return Err(TreeError::CyclicParent);
            }
        }

        let id = self.next_id;
        self.next_id += 1;

        let task = Task {
            id,
            parent_id,
            name: name.to_string(),
            is_project,
            is_payable,
            is_archived: false,
            total_duration_secs: 0,
        };

        self.tasks.insert(id, task);
        Ok(id)
    }

    pub fn get_task(&self, id: i64) -> Option<&Task> {
        self.tasks.get(&id)
    }

    pub fn get_task_mut(&mut self, id: i64) -> Option<&mut Task> {
        self.tasks.get_mut(&id)
    }

    pub fn remove_task(&mut self, id: i64) -> Result<(), TreeError> {
        if !self.tasks.contains_key(&id) {
            return Err(TreeError::NotFound(id));
        }

        let child_ids: Vec<i64> = self
            .tasks
            .values()
            .filter(|t| t.parent_id == Some(id))
            .map(|t| t.id)
            .collect();

        for child_id in child_ids {
            self.remove_task(child_id)?;
        }

        self.tasks.remove(&id);
        Ok(())
    }

    pub fn rename_task(&mut self, id: i64, new_name: &str) -> Result<(), TreeError> {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            return Err(TreeError::DuplicateName);
        }

        let task = self.tasks.get(&id).ok_or(TreeError::NotFound(id))?;
        let parent_id = task.parent_id;

        if self
            .tasks
            .values()
            .any(|t| t.name == new_name && t.parent_id == parent_id && t.id != id)
        {
            return Err(TreeError::DuplicateName);
        }

        if let Some(task) = self.tasks.get_mut(&id) {
            task.name = new_name.to_string();
        }
        Ok(())
    }

    pub fn archive_task(&mut self, id: i64) -> Result<(), TreeError> {
        let task = self.tasks.get_mut(&id).ok_or(TreeError::NotFound(id))?;
        task.is_archived = true;
        Ok(())
    }

    pub fn unarchive_task(&mut self, id: i64) -> Result<(), TreeError> {
        let task = self.tasks.get_mut(&id).ok_or(TreeError::NotFound(id))?;
        task.is_archived = false;
        Ok(())
    }

    pub fn children_of(&self, parent_id: i64) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|t| t.parent_id == Some(parent_id))
            .collect()
    }

    pub fn root_tasks(&self) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|t| t.parent_id.is_none())
            .collect()
    }

    pub fn descendants(&self, task_id: i64) -> Vec<i64> {
        let mut result = Vec::new();
        let mut stack = vec![task_id];

        while let Some(current) = stack.pop() {
            let children: Vec<i64> = self
                .tasks
                .values()
                .filter(|t| t.parent_id == Some(current))
                .map(|t| t.id)
                .collect();
            for child in children {
                result.push(child);
                stack.push(child);
            }
        }
        result
    }

    pub fn path_to_root(&self, task_id: i64) -> Vec<i64> {
        let mut path = Vec::new();
        let mut current = task_id;
        loop {
            path.push(current);
            match self.tasks.get(&current) {
                Some(task) if task.parent_id.is_some() => {
                    current = task.parent_id.unwrap();
                }
                _ => break,
            }
        }
        path
    }

    pub fn all_tasks(&self) -> Vec<&Task> {
        self.tasks.values().collect()
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn would_create_cycle(&self, parent_id: i64, child_id: i64) -> bool {
        let mut current = parent_id;
        loop {
            if current == child_id {
                return true;
            }
            match self.tasks.get(&current) {
                Some(task) if task.parent_id.is_some() => {
                    current = task.parent_id.unwrap();
                }
                _ => return false,
            }
        }
    }

    pub fn update_duration(&mut self, task_id: i64, additional_secs: u64) -> Result<(), TreeError> {
        let task = self
            .tasks
            .get_mut(&task_id)
            .ok_or(TreeError::NotFound(task_id))?;
        task.total_duration_secs += additional_secs;
        Ok(())
    }

    pub fn cumulative_duration(&self, task_id: i64) -> Result<u64, TreeError> {
        if !self.tasks.contains_key(&task_id) {
            return Err(TreeError::NotFound(task_id));
        }
        let mut total = 0u64;
        let mut stack = vec![task_id];
        while let Some(current) = stack.pop() {
            if let Some(task) = self.tasks.get(&current) {
                total += task.total_duration_secs;
            }
            let children: Vec<i64> = self
                .tasks
                .values()
                .filter(|t| t.parent_id == Some(current))
                .map(|t| t.id)
                .collect();
            stack.extend(children);
        }
        Ok(total)
    }
}

impl Default for TaskTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tree_is_empty() {
        let tree = TaskTree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
    }

    #[test]
    fn add_root_task() {
        let mut tree = TaskTree::new();
        let id = tree.add_task(None, "Project Alpha", true, true).unwrap();
        assert_eq!(tree.len(), 1);
        let task = tree.get_task(id).unwrap();
        assert_eq!(task.name, "Project Alpha");
        assert!(task.is_project);
        assert!(task.is_payable);
        assert!(task.parent_id.is_none());
    }

    #[test]
    fn add_child_task() {
        let mut tree = TaskTree::new();
        let parent_id = tree.add_task(None, "Project", true, true).unwrap();
        let child_id = tree
            .add_task(Some(parent_id), "Subtask", false, true)
            .unwrap();
        let child = tree.get_task(child_id).unwrap();
        assert_eq!(child.parent_id, Some(parent_id));
    }

    #[test]
    fn reject_empty_name() {
        let mut tree = TaskTree::new();
        assert!(tree.add_task(None, "  ", true, true).is_err());
    }

    #[test]
    fn reject_duplicate_name_same_parent() {
        let mut tree = TaskTree::new();
        tree.add_task(None, "Task", true, true).unwrap();
        assert!(tree.add_task(None, "Task", true, true).is_err());
    }

    #[test]
    fn allow_duplicate_name_different_parent() {
        let mut tree = TaskTree::new();
        let p1 = tree.add_task(None, "Project 1", true, true).unwrap();
        let p2 = tree.add_task(None, "Project 2", true, true).unwrap();
        tree.add_task(Some(p1), "Same Name", false, true).unwrap();
        tree.add_task(Some(p2), "Same Name", false, true).unwrap();
        assert_eq!(tree.len(), 4);
    }

    #[test]
    fn reject_invalid_parent() {
        let mut tree = TaskTree::new();
        assert!(tree.add_task(Some(999), "Orphan", false, true).is_err());
    }

    #[test]
    fn reject_cyclic_parent() {
        let mut tree = TaskTree::new();
        let a = tree.add_task(None, "A", true, true).unwrap();
        let b = tree.add_task(Some(a), "B", true, true).unwrap();
        assert!(
            tree.would_create_cycle(b, a),
            "making B a parent of A should create a cycle"
        );
    }

    #[test]
    fn add_deeply_nested_task() {
        let mut tree = TaskTree::new();
        let a = tree.add_task(None, "A", true, true).unwrap();
        let b = tree.add_task(Some(a), "B", true, true).unwrap();
        let c = tree.add_task(Some(b), "C", true, true).unwrap();
        let d = tree.add_task(Some(c), "D", true, true).unwrap();
        assert_eq!(tree.len(), 4);
        assert_eq!(tree.path_to_root(d), vec![d, c, b, a]);
    }

    #[test]
    fn remove_task_cascades() {
        let mut tree = TaskTree::new();
        let parent = tree.add_task(None, "Parent", true, true).unwrap();
        let child = tree.add_task(Some(parent), "Child", false, true).unwrap();
        tree.add_task(Some(child), "Grandchild", false, true)
            .unwrap();
        assert_eq!(tree.len(), 3);
        tree.remove_task(parent).unwrap();
        assert!(tree.is_empty());
    }

    #[test]
    fn remove_nonexistent_task() {
        let mut tree = TaskTree::new();
        assert!(tree.remove_task(42).is_err());
    }

    #[test]
    fn rename_task() {
        let mut tree = TaskTree::new();
        let id = tree.add_task(None, "Old Name", true, true).unwrap();
        tree.rename_task(id, "New Name").unwrap();
        assert_eq!(tree.get_task(id).unwrap().name, "New Name");
    }

    #[test]
    fn rename_to_empty_fails() {
        let mut tree = TaskTree::new();
        let id = tree.add_task(None, "Task", true, true).unwrap();
        assert!(tree.rename_task(id, "").is_err());
    }

    #[test]
    fn archive_and_unarchive() {
        let mut tree = TaskTree::new();
        let id = tree.add_task(None, "Task", true, true).unwrap();
        tree.archive_task(id).unwrap();
        assert!(tree.get_task(id).unwrap().is_archived);
        tree.unarchive_task(id).unwrap();
        assert!(!tree.get_task(id).unwrap().is_archived);
    }

    #[test]
    fn children_of() {
        let mut tree = TaskTree::new();
        let p = tree.add_task(None, "P", true, true).unwrap();
        let c1 = tree.add_task(Some(p), "C1", false, true).unwrap();
        let c2 = tree.add_task(Some(p), "C2", false, true).unwrap();
        tree.add_task(None, "Other", false, true).unwrap();
        let children = tree.children_of(p);
        let mut child_ids: Vec<i64> = children.iter().map(|t| t.id).collect();
        child_ids.sort();
        let mut expected = vec![c1, c2];
        expected.sort();
        assert_eq!(child_ids, expected);
    }

    #[test]
    fn root_tasks() {
        let mut tree = TaskTree::new();
        let r1 = tree.add_task(None, "Root 1", true, true).unwrap();
        let r2 = tree.add_task(None, "Root 2", true, true).unwrap();
        let p = tree.add_task(Some(r1), "Child", false, true).unwrap();
        let roots: Vec<i64> = tree.root_tasks().iter().map(|t| t.id).collect();
        assert!(roots.contains(&r1));
        assert!(roots.contains(&r2));
        assert!(!roots.contains(&p));
    }

    #[test]
    fn descendants() {
        let mut tree = TaskTree::new();
        let a = tree.add_task(None, "A", true, true).unwrap();
        let b = tree.add_task(Some(a), "B", true, true).unwrap();
        let c = tree.add_task(Some(b), "C", false, true).unwrap();
        let d = tree.add_task(Some(a), "D", false, true).unwrap();
        let desc = tree.descendants(a);
        assert!(desc.contains(&b));
        assert!(desc.contains(&c));
        assert!(desc.contains(&d));
        assert_eq!(desc.len(), 3);
    }

    #[test]
    fn path_to_root() {
        let mut tree = TaskTree::new();
        let a = tree.add_task(None, "A", true, true).unwrap();
        let b = tree.add_task(Some(a), "B", true, true).unwrap();
        let c = tree.add_task(Some(b), "C", false, true).unwrap();
        let path = tree.path_to_root(c);
        assert_eq!(path, vec![c, b, a]);
    }

    #[test]
    fn update_and_cumulative_duration() {
        let mut tree = TaskTree::new();
        let p = tree.add_task(None, "Project", true, true).unwrap();
        let t1 = tree.add_task(Some(p), "Task 1", false, true).unwrap();
        let t2 = tree.add_task(Some(p), "Task 2", false, true).unwrap();

        tree.update_duration(t1, 3600).unwrap();
        tree.update_duration(t2, 1800).unwrap();
        tree.update_duration(p, 600).unwrap();

        assert_eq!(tree.cumulative_duration(t1).unwrap(), 3600);
        assert_eq!(tree.cumulative_duration(t2).unwrap(), 1800);
        assert_eq!(tree.cumulative_duration(p).unwrap(), 6000);
    }
}
