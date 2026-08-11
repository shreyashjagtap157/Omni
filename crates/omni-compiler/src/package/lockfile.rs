use super::solver::{LockfileData, PackageName, Version};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// A lockfile that can be read from and written to disk.
#[derive(Debug, Clone)]
pub struct Lockfile {
    pub data: LockfileData,
}

impl Lockfile {
    /// Create a new lockfile from resolved dependencies.
    pub fn new(resolved: BTreeMap<PackageName, Version>) -> Self {
        let mut packages: BTreeMap<String, super::solver::PackageLockEntry> = BTreeMap::new();
        let mut root: BTreeMap<String, String> = BTreeMap::new();

        for (name, version) in resolved {
            packages.insert(
                name.0.clone(),
                super::solver::PackageLockEntry {
                    version: version.0.clone(),
                    source: "registry".to_string(),
                    dependencies: BTreeMap::new(),
                    checksum: None,
                },
            );
            root.insert(name.0.clone(), version.0.clone());
        }

        Self {
            data: LockfileData {
                version: 1,
                root,
                packages,
            },
        }
    }

    /// Create a lockfile from full lockfile data (with dependencies).
    pub fn from_data(data: LockfileData) -> Self {
        Self { data }
    }

    /// Write the lockfile to a file in TOML format.
    pub fn write_to_file(&self, path: &Path) -> std::io::Result<()> {
        let content = self.data.to_toml();
        let mut file = File::create(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Write the lockfile to a file in JSON format.
    pub fn write_to_file_json(&self, path: &Path) -> Result<(), String> {
        let content = self.data.to_json()?;
        std::fs::write(path, content).map_err(|e| e.to_string())
    }

    /// Read a lockfile from a TOML file.
    pub fn read_from_file(path: &Path) -> Result<Self, String> {
        let mut content = String::new();
        File::open(path)
            .map_err(|e| format!("Failed to open lockfile: {}", e))?
            .read_to_string(&mut content)
            .map_err(|e| format!("Failed to read lockfile: {}", e))?;

        Self::parse_toml(&content)
    }

    /// Parse a lockfile from TOML content.
    pub fn parse_toml(content: &str) -> Result<Self, String> {
        let mut root: BTreeMap<String, String> = BTreeMap::new();
        let mut packages: BTreeMap<String, super::solver::PackageLockEntry> = BTreeMap::new();

        let mut in_root = false;
        let mut in_packages = false;
        let mut current_pkg: Option<super::solver::PackageLockEntry> = None;
        let mut current_pkg_name: Option<String> = None;

        let mut collecting_deps: Option<String> = None;

        for line in content.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                if let Some(ref mut buf) = collecting_deps {
                    buf.push(' ');
                    buf.push_str(line);
                }
                continue;
            }

            if let Some(ref mut buf) = collecting_deps {
                buf.push(' ');
                buf.push_str(line);
                if line.contains('}') {
                    let full_value = buf.clone();
                    collecting_deps = None;
                    if let Some(ref mut pkg) = current_pkg {
                        Self::parse_deps_inline(&full_value, &mut pkg.dependencies);
                    }
                }
                continue;
            }

            if line == "[root]" {
                in_root = true;
                in_packages = false;
                continue;
            }

            if line == "[[packages]]" {
                if let (Some(name), Some(pkg)) = (current_pkg_name.take(), current_pkg.take()) {
                    packages.insert(name, pkg);
                }
                in_root = false;
                in_packages = true;
                current_pkg = Some(super::solver::PackageLockEntry {
                    version: String::new(),
                    source: String::new(),
                    dependencies: BTreeMap::new(),
                    checksum: None,
                });
                continue;
            }

            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                let value = line[eq_pos + 1..].trim();

                if in_root {
                    root.insert(key.to_string(), unquote(value));
                } else if in_packages {
                    if let Some(ref mut pkg) = current_pkg {
                        match key {
                            "name" => current_pkg_name = Some(unquote(value)),
                            "version" => pkg.version = unquote(value),
                            "source" => pkg.source = unquote(value),
                            "checksum" => pkg.checksum = Some(unquote(value)),
                            "dependencies" => {
                                if value.starts_with('{') && value.ends_with('}') {
                                    Self::parse_deps_inline(value, &mut pkg.dependencies);
                                } else if value.starts_with('{') {
                                    collecting_deps = Some(value.to_string());
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if let (Some(name), Some(pkg)) = (current_pkg_name.take(), current_pkg.take()) {
            packages.insert(name, pkg);
        }

        Ok(Self {
            data: LockfileData {
                version: 1,
                root,
                packages,
            },
        })
    }

    fn parse_deps_inline(value: &str, deps: &mut BTreeMap<String, String>) {
        let inner = if value.starts_with('{') && value.ends_with('}') {
            &value[1..value.len() - 1]
        } else {
            value
        };
        for part in inner.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            if let Some(eq) = part.find('=') {
                let dep_name = unquote(part[..eq].trim());
                let dep_constraint = unquote(part[eq + 1..].trim());
                deps.insert(dep_name, dep_constraint);
            }
        }
    }

    /// Check if the lockfile contains a specific package.
    pub fn contains_package(&self, name: &str) -> bool {
        self.data.packages.contains_key(name)
    }

    /// Get the version of a package in the lockfile.
    pub fn get_package_version(&self, name: &str) -> Option<&str> {
        self.data.packages.get(name).map(|e| e.version.as_str())
    }

    /// Get all package names in the lockfile.
    pub fn package_names(&self) -> Vec<&str> {
        self.data.packages.keys().map(|s| s.as_str()).collect()
    }

    /// Get the number of packages in the lockfile.
    pub fn package_count(&self) -> usize {
        self.data.packages.len()
    }

    /// Check if this lockfile is empty (no packages).
    pub fn is_empty(&self) -> bool {
        self.data.packages.is_empty()
    }

    /// Merge another lockfile into this one.
    /// Packages from the other lockfile override existing ones.
    pub fn merge(&mut self, other: &Lockfile) {
        for (name, entry) in &other.data.packages {
            self.data.packages.insert(name.clone(), entry.clone());
        }
        for (name, constraint) in &other.data.root {
            self.data.root.insert(name.clone(), constraint.clone());
        }
    }

    /// Compute a deterministic checksum of the lockfile content.
    /// Useful for detecting if dependencies have changed.
    pub fn content_hash(&self) -> String {
        let toml = self.data.to_toml();
        format!("{:x}", md5_hash(toml.as_bytes()))
    }
}

fn unquote(s: &str) -> String {
    s.trim_matches('"').trim_matches('\'').to_string()
}

/// Simple hash function for content hashing (not cryptographic).
fn md5_hash(bytes: &[u8]) -> u128 {
    // Simple FNV-1a based hash for deterministic output
    let mut hash: u128 = 0xcbf29ce484222325;
    for &byte in bytes {
        hash ^= byte as u128;
        hash = hash.wrapping_mul(0x10000000000000000 + 0x1b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_lockfile_new() {
        let mut resolved = BTreeMap::new();
        resolved.insert(PackageName("foo".to_string()), Version("1.0.0".to_string()));
        resolved.insert(PackageName("bar".to_string()), Version("2.0.0".to_string()));

        let lockfile = Lockfile::new(resolved);
        assert_eq!(lockfile.package_count(), 2);
        assert!(lockfile.contains_package("foo"));
        assert!(lockfile.contains_package("bar"));
        assert_eq!(lockfile.get_package_version("foo"), Some("1.0.0"));
    }

    #[test]
    fn test_lockfile_toml_roundtrip() {
        let toml_input = r#"# This file is automatically generated by the Omni package manager.
# Do not edit manually.

lock_version = 1

[root]
foo = "1.0.0"

[[packages]]
name = "foo"
version = "1.0.0"
source = "registry"
"#;

        let lockfile = Lockfile::parse_toml(toml_input).unwrap();
        assert!(lockfile.contains_package("foo"));
        assert_eq!(lockfile.get_package_version("foo"), Some("1.0.0"));

        // Roundtrip: serialize and parse again
        let serialized = lockfile.data.to_toml();
        let lockfile2 = Lockfile::parse_toml(&serialized).unwrap();
        assert_eq!(lockfile.package_count(), lockfile2.package_count());
    }

    #[test]
    fn test_lockfile_with_dependencies() {
        let toml_input = r#"# This file is automatically generated by the Omni package manager.
# Do not edit manually.

lock_version = 1

[root]
app = "^1.0.0"

[[packages]]
name = "app"
version = "1.0.0"
source = "registry"
dependencies = {
  "lib" = "^1.0.0",
}

[[packages]]
name = "lib"
version = "1.2.0"
source = "registry"
"#;

        let lockfile = Lockfile::parse_toml(toml_input).unwrap();
        assert_eq!(lockfile.package_count(), 2);

        let app_entry = lockfile.data.packages.get("app").unwrap();
        assert_eq!(
            app_entry.dependencies.get("lib"),
            Some(&"^1.0.0".to_string())
        );
    }

    #[test]
    fn test_lockfile_merge() {
        let mut resolved1 = BTreeMap::new();
        resolved1.insert(PackageName("foo".to_string()), Version("1.0.0".to_string()));
        let mut lockfile1 = Lockfile::new(resolved1);

        let mut resolved2 = BTreeMap::new();
        resolved2.insert(PackageName("bar".to_string()), Version("2.0.0".to_string()));
        let lockfile2 = Lockfile::new(resolved2);

        lockfile1.merge(&lockfile2);
        assert_eq!(lockfile1.package_count(), 2);
        assert!(lockfile1.contains_package("foo"));
        assert!(lockfile1.contains_package("bar"));
    }

    #[test]
    fn test_lockfile_empty() {
        let resolved = BTreeMap::new();
        let lockfile = Lockfile::new(resolved);
        assert!(lockfile.is_empty());
        assert_eq!(lockfile.package_count(), 0);
    }

    #[test]
    fn test_lockfile_content_hash_deterministic() {
        let mut resolved = BTreeMap::new();
        resolved.insert(PackageName("foo".to_string()), Version("1.0.0".to_string()));

        let lockfile1 = Lockfile::new(resolved.clone());
        let lockfile2 = Lockfile::new(resolved);

        // Same content should produce same hash
        assert_eq!(lockfile1.content_hash(), lockfile2.content_hash());
    }

    #[test]
    fn test_lockfile_package_names() {
        let mut resolved = BTreeMap::new();
        resolved.insert(
            PackageName("alpha".to_string()),
            Version("1.0.0".to_string()),
        );
        resolved.insert(
            PackageName("beta".to_string()),
            Version("2.0.0".to_string()),
        );
        resolved.insert(
            PackageName("gamma".to_string()),
            Version("3.0.0".to_string()),
        );

        let lockfile = Lockfile::new(resolved);
        let names = lockfile.package_names();
        assert_eq!(names.len(), 3);
        // BTreeMap ensures sorted order
        assert_eq!(names[0], "alpha");
        assert_eq!(names[1], "beta");
        assert_eq!(names[2], "gamma");
    }

    #[test]
    fn test_lockfile_write_and_read_file() {
        let mut resolved = BTreeMap::new();
        resolved.insert(
            PackageName("test-pkg".to_string()),
            Version("1.0.0".to_string()),
        );
        let lockfile = Lockfile::new(resolved);

        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("omni_test_lockfile.toml");

        lockfile.write_to_file(&path).unwrap();
        let read_back = Lockfile::read_from_file(&path).unwrap();

        assert!(read_back.contains_package("test-pkg"));
        assert_eq!(read_back.get_package_version("test-pkg"), Some("1.0.0"));

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }
}
