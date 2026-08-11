use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct Node {
    pub path: PathBuf,
    pub last_modified: Option<SystemTime>,
    pub dependencies: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct BuildGraph {
    pub nodes: HashMap<PathBuf, Node>,
}

impl BuildGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, path: PathBuf, dependencies: Vec<PathBuf>) {
        let last_modified = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());

        self.nodes.insert(
            path.clone(),
            Node {
                path,
                last_modified,
                dependencies,
            },
        );
    }

    pub fn is_stale(&self, target: &Path) -> bool {
        let Some(node) = self.nodes.get(target) else {
            // Target not in graph means it needs building
            return true;
        };

        let Some(target_time) = node.last_modified else {
            // Target file doesn't exist or we can't get modified time
            return true;
        };

        for dep in &node.dependencies {
            if let Some(dep_node) = self.nodes.get(dep) {
                if let Some(cached_time) = dep_node.last_modified {
                    let fresh_time = match std::fs::metadata(dep).and_then(|m| m.modified()) {
                        Ok(time) => time,
                        Err(_) => return true,
                    };
                    if fresh_time > target_time || fresh_time != cached_time {
                        return true;
                    }
                } else {
                    // Dependency doesn't exist, we should rebuild
                    return true;
                }
            } else {
                // Dependency not in graph, check filesystem directly
                if let Ok(meta) = std::fs::metadata(dep) {
                    if let Ok(dep_time) = meta.modified() {
                        if dep_time > target_time {
                            return true;
                        }
                    } else {
                        return true;
                    }
                } else {
                    return true;
                }
            }
        }
        false
    }
}
