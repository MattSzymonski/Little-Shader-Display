use std::{collections::HashMap, fs, path::{Path, PathBuf}, time::SystemTime};


pub struct FileWatcher {
    path: PathBuf,
    previous_metadata: HashMap<PathBuf, SystemTime>,
}

impl FileWatcher {
    pub fn new(path: PathBuf) -> Self {
        let previous_metadata = Self::get_file_metadata(&path);
        Self { path, previous_metadata }
    }

    fn get_file_metadata(path: &Path) -> HashMap<PathBuf, SystemTime> {
        let mut file_metadata = HashMap::new();

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if let Ok(metadata) = path.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Some(file_name) = path.file_name() {
                            if let Some(file_name_str) = file_name.to_str() {
                                file_metadata.insert(path, modified);
                            }
                        }
                    }
                }
            }
        }

        file_metadata
    }

    fn check_for_changes(&mut self) -> Vec<PathBuf> {
        let current_metadata = Self::get_file_metadata(&self.path);
        let mut changes = Vec::new();

        // Check for modified or new files
        for (file_path, modified_time) in &current_metadata {
            match self.previous_metadata.get(file_path) {
                Some(&prev_time) if prev_time != *modified_time => {
                    changes.push(file_path.clone());
                }
                Some(_) => {
                    // Do nothing for now, you can add code here if needed
                }
                None => {
                    changes.push(file_path.clone());
                }
            }
        }

        // Check for removed files
        for file_path in self.previous_metadata.keys() {
            if !current_metadata.contains_key(file_path) {
                changes.push(file_path.clone());
            }
        }

        // Update previous metadata for the next iteration
        self.previous_metadata = current_metadata;

        changes
    }

    pub fn get_changes(&mut self) -> Option<Vec<PathBuf>> {
        let changes = self.check_for_changes();
        if !changes.is_empty() {
            Some(changes.clone())
        } else {
            None
        }
    }
}