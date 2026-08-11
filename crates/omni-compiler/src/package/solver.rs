use std::collections::{BTreeMap, HashMap, HashSet};

/// A package name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageName(pub String);

impl std::fmt::Display for PackageName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A semantic version (simplified as a string for now, but parsed for comparison).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(pub String);

impl Version {
    /// Parse a version string into components for comparison.
    fn components(&self) -> Vec<u64> {
        self.0
            .split('.')
            .filter_map(|s| s.parse::<u64>().ok())
            .collect()
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A version constraint (supports semver-like ranges).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VersionConstraint(pub String);

impl VersionConstraint {
    /// Check if a version satisfies this constraint.
    /// Supports: "*", "1.0.0", "^1.0.0", ">=1.0.0", "<2.0.0", ">=1.0.0, <2.0.0"
    pub fn is_satisfied_by(&self, version: &Version) -> bool {
        let constraint = self.0.trim();

        if constraint == "*" {
            return true;
        }

        // Handle comma-separated constraints (AND)
        if constraint.contains(',') {
            return constraint
                .split(',')
                .map(|s| s.trim())
                .all(|part| VersionConstraint(part.to_string()).is_satisfied_by(version));
        }

        let ver = version.components();

        // Caret constraint: ^1.2.3 means >=1.2.3, <2.0.0
        if let Some(base) = constraint.strip_prefix('^') {
            let base = base.trim();
            let base_ver = Version(base.to_string());
            let base_components = base_ver.components();

            if ver < base_components {
                return false;
            }

            // Upper bound: next major version
            if let Some(&major) = base_components.first() {
                let upper: Vec<u64> = vec![major + 1, 0, 0];
                if ver >= upper {
                    return false;
                }
            }
            return true;
        }

        // Tilde constraint: ~1.2.3 means >=1.2.3, <1.3.0
        if let Some(base) = constraint.strip_prefix('~') {
            let base = base.trim();
            let base_ver = Version(base.to_string());
            let base_components = base_ver.components();

            if ver < base_components {
                return false;
            }

            if base_components.len() >= 2 {
                let upper: Vec<u64> = vec![base_components[0], base_components[1] + 1, 0];
                if ver >= upper {
                    return false;
                }
            }
            return true;
        }

        // Comparison operators
        if let Some(target) = constraint.strip_prefix(">=") {
            let target = Version(target.trim().to_string());
            return ver >= target.components();
        }
        if let Some(target) = constraint.strip_prefix('>') {
            let target = Version(target.trim().to_string());
            return ver > target.components();
        }
        if let Some(target) = constraint.strip_prefix("<=") {
            let target = Version(target.trim().to_string());
            return ver <= target.components();
        }
        if let Some(target) = constraint.strip_prefix('<') {
            let target = Version(target.trim().to_string());
            return ver < target.components();
        }
        if let Some(target) = constraint.strip_prefix('=') {
            let target = Version(target.trim().to_string());
            return ver == target.components();
        }

        // Exact match
        let target = Version(constraint.to_string());
        ver == target.components()
    }
}

impl std::fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A specific version of a package with its dependencies.
#[derive(Debug, Clone)]
pub struct PackageVersion {
    pub name: PackageName,
    pub version: Version,
    pub dependencies: Vec<DependencyRequirement>,
}

/// A dependency requirement: package name + version constraint.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DependencyRequirement {
    pub name: PackageName,
    pub constraint: VersionConstraint,
}

impl std::fmt::Display for DependencyRequirement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.name, self.constraint)
    }
}

/// A registry of available package versions.
pub trait Registry {
    fn available_versions(&self, name: &PackageName) -> Vec<PackageVersion>;
    fn all_package_names(&self) -> Vec<PackageName>;
}

/// A mock registry for testing.
pub struct MockRegistry {
    pub data: HashMap<PackageName, Vec<PackageVersion>>,
}

impl MockRegistry {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn add_package(&mut self, name: PackageName, versions: Vec<PackageVersion>) {
        self.data.insert(name, versions);
    }
}

impl Default for MockRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry for MockRegistry {
    fn available_versions(&self, name: &PackageName) -> Vec<PackageVersion> {
        self.data.get(name).cloned().unwrap_or_default()
    }

    fn all_package_names(&self) -> Vec<PackageName> {
        self.data.keys().cloned().collect()
    }
}

/// Errors that can occur during dependency resolution.
#[derive(Debug, thiserror::Error)]
pub enum SolverError {
    #[error("No valid version found for package {0}")]
    NoValidVersion(PackageName),
    #[error("Package {0} not found in registry")]
    PackageNotFound(PackageName),
    #[error("Dependency conflict: {0}")]
    Conflict(String),
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),
}

/// A term in the PubGrub algorithm representing a package version constraint.
#[derive(Debug, Clone)]
pub struct Term {
    pub package: PackageName,
    pub constraint: VersionConstraint,
    pub is_positive: bool,
}

impl Term {
    pub fn positive(package: PackageName, constraint: VersionConstraint) -> Self {
        Self {
            package,
            constraint,
            is_positive: true,
        }
    }

    pub fn negative(package: PackageName, constraint: VersionConstraint) -> Self {
        Self {
            package,
            constraint,
            is_positive: false,
        }
    }
}

/// An assignment in the PubGrub derivation tree.
#[derive(Debug, Clone)]
pub enum Assignment {
    /// A decision: we chose a specific version for a package.
    Decision {
        package: PackageName,
        version: Version,
    },
    /// A derivation: inferred from other constraints.
    Derivation {
        package: PackageName,
        constraint: VersionConstraint,
        cause: Box<DerivationCause>,
    },
}

/// The cause of a derivation.
#[derive(Debug, Clone)]
pub enum DerivationCause {
    /// Root dependency.
    RootDependency,
    /// Dependency of another package.
    DependencyOf(PackageName, Version),
    /// Conflict between two constraints.
    Conflict(Term, Term),
}

/// The PubGrub dependency solver.
/// Implements a simplified version of the PubGrub algorithm for deterministic
/// dependency resolution with lockfile generation.
pub struct PubGrubSolver<'a> {
    registry: &'a dyn Registry,
}

impl<'a> PubGrubSolver<'a> {
    pub fn new(registry: &'a dyn Registry) -> Self {
        Self { registry }
    }

    /// Resolve dependencies to a concrete set of package versions.
    /// Returns a map of package name -> resolved version.
    pub fn resolve(
        &self,
        root_requirements: Vec<DependencyRequirement>,
    ) -> Result<HashMap<PackageName, Version>, SolverError> {
        // Use a BTreeMap internally for deterministic ordering
        let mut solutions: BTreeMap<PackageName, Version> = BTreeMap::new();
        let mut constraints: HashMap<PackageName, Vec<VersionConstraint>> = HashMap::new();

        // Add root requirements as initial constraints
        for req in &root_requirements {
            constraints
                .entry(req.name.clone())
                .or_default()
                .push(req.constraint.clone());
        }

        // Process queue of requirements
        let mut queue: Vec<DependencyRequirement> = root_requirements;
        let mut visited: HashSet<(PackageName, VersionConstraint)> = HashSet::new();

        while let Some(req) = queue.pop() {
            let key = (req.name.clone(), req.constraint.clone());
            if visited.contains(&key) {
                continue;
            }
            visited.insert(key);

            // Check if already resolved
            if let Some(resolved_version) = solutions.get(&req.name) {
                if !req.constraint.is_satisfied_by(resolved_version) {
                    // Conflict: existing version doesn't satisfy new constraint
                    let empty_constraints = Vec::new();
                    let existing_constraints =
                        constraints.get(&req.name).unwrap_or(&empty_constraints);
                    let constraint_strs: Vec<String> = existing_constraints
                        .iter()
                        .map(|c| c.0.clone())
                        .chain(std::iter::once(req.constraint.0.clone()))
                        .collect();
                    return Err(SolverError::Conflict(format!(
                        "Package {} version {} does not satisfy constraint {} (also required: {})",
                        req.name,
                        resolved_version,
                        req.constraint,
                        constraint_strs.join(", ")
                    )));
                }
                continue;
            }

            // Find the best version that satisfies all accumulated constraints
            let all_constraints = constraints.get(&req.name).cloned().unwrap_or_default();

            let best_version = self.find_best_version(&req.name, &all_constraints)?;

            // Record the solution
            solutions.insert(req.name.clone(), best_version.version.clone());

            // Add transitive dependencies to the queue
            for dep in &best_version.dependencies {
                constraints
                    .entry(dep.name.clone())
                    .or_default()
                    .push(dep.constraint.clone());
                queue.push(dep.clone());
            }
        }

        // Convert BTreeMap to HashMap for the return type
        Ok(solutions.into_iter().collect())
    }

    /// Find the best (highest) version that satisfies all constraints.
    fn find_best_version(
        &self,
        name: &PackageName,
        constraints: &[VersionConstraint],
    ) -> Result<PackageVersion, SolverError> {
        let versions = self.registry.available_versions(name);
        if versions.is_empty() {
            return Err(SolverError::PackageNotFound(name.clone()));
        }

        // Filter versions that satisfy ALL constraints
        let mut valid: Vec<&PackageVersion> = versions
            .iter()
            .filter(|v| constraints.iter().all(|c| c.is_satisfied_by(&v.version)))
            .collect();

        if valid.is_empty() {
            let _constraint_strs: Vec<String> = constraints.iter().map(|c| c.0.clone()).collect();
            return Err(SolverError::NoValidVersion(name.clone()));
        }

        // Sort by version descending and pick the highest
        valid.sort_by(|a, b| b.version.cmp(&a.version));
        Ok(valid[0].clone())
    }

    /// Resolve and produce a deterministic lockfile representation.
    /// Returns a sorted list of (package, version, dependencies) tuples.
    pub fn resolve_with_lock(
        &self,
        root_requirements: Vec<DependencyRequirement>,
    ) -> Result<LockfileData, SolverError> {
        let solutions = self.resolve(root_requirements.clone())?;

        let mut packages: BTreeMap<String, PackageLockEntry> = BTreeMap::new();

        // Build the lockfile entries
        for (name, version) in &solutions {
            let versions = self.registry.available_versions(name);
            if let Some(pkg_ver) = versions.iter().find(|v| v.version == *version) {
                let deps: BTreeMap<String, String> = pkg_ver
                    .dependencies
                    .iter()
                    .map(|d| (d.name.0.clone(), d.constraint.0.clone()))
                    .collect();

                packages.insert(
                    name.0.clone(),
                    PackageLockEntry {
                        version: version.0.clone(),
                        source: "registry".to_string(),
                        dependencies: deps,
                        checksum: None,
                    },
                );
            }
        }

        // Root dependencies
        let root_deps: BTreeMap<String, String> = root_requirements
            .iter()
            .map(|r| (r.name.0.clone(), r.constraint.0.clone()))
            .collect();

        Ok(LockfileData {
            version: 1,
            root: root_deps,
            packages,
        })
    }
}

/// Lockfile data structure for serialization.
#[derive(Debug, Clone)]
pub struct LockfileData {
    pub version: u32,
    pub root: BTreeMap<String, String>,
    pub packages: BTreeMap<String, PackageLockEntry>,
}

/// A single package entry in the lockfile.
#[derive(Debug, Clone)]
pub struct PackageLockEntry {
    pub version: String,
    pub source: String,
    pub dependencies: BTreeMap<String, String>,
    pub checksum: Option<String>,
}

impl LockfileData {
    /// Serialize to TOML format (deterministic output).
    pub fn to_toml(&self) -> String {
        let mut output = String::new();
        output.push_str("# This file is automatically generated by the Omni package manager.\n");
        output.push_str("# Do not edit manually.\n\n");
        output.push_str(&format!("lock_version = {}\n\n", self.version));

        output.push_str("[root]\n");
        for (name, constraint) in &self.root {
            output.push_str(&format!("{} = \"{}\"\n", name, constraint));
        }
        output.push('\n');

        for (name, entry) in &self.packages {
            output.push_str("[[packages]]\n");
            output.push_str(&format!("name = \"{}\"\n", name));
            output.push_str(&format!("version = \"{}\"\n", entry.version));
            output.push_str(&format!("source = \"{}\"\n", entry.source));

            if !entry.dependencies.is_empty() {
                output.push_str("dependencies = {\n");
                for (dep_name, dep_constraint) in &entry.dependencies {
                    output.push_str(&format!("  \"{}\" = \"{}\",\n", dep_name, dep_constraint));
                }
                output.push_str("}\n");
            }

            if let Some(ref checksum) = entry.checksum {
                output.push_str(&format!("checksum = \"{}\"\n", checksum));
            }

            output.push('\n');
        }

        output
    }

    /// Serialize to JSON format.
    pub fn to_json(&self) -> Result<String, String> {
        #[derive(serde::Serialize)]
        struct JsonLockfile<'a> {
            lock_version: u32,
            root: &'a BTreeMap<String, String>,
            packages: &'a BTreeMap<String, JsonPackageEntry<'a>>,
        }

        #[derive(serde::Serialize)]
        struct JsonPackageEntry<'a> {
            version: &'a str,
            source: &'a str,
            dependencies: &'a BTreeMap<String, String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            checksum: &'a Option<String>,
        }

        let json_packages: BTreeMap<String, JsonPackageEntry> = self
            .packages
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    JsonPackageEntry {
                        version: &v.version,
                        source: &v.source,
                        dependencies: &v.dependencies,
                        checksum: &v.checksum,
                    },
                )
            })
            .collect();

        let jf = JsonLockfile {
            lock_version: self.version,
            root: &self.root,
            packages: &json_packages,
        };

        serde_json::to_string_pretty(&jf).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_constraint_exact() {
        let c = VersionConstraint("1.0.0".to_string());
        assert!(c.is_satisfied_by(&Version("1.0.0".to_string())));
        assert!(!c.is_satisfied_by(&Version("1.0.1".to_string())));
    }

    #[test]
    fn test_version_constraint_wildcard() {
        let c = VersionConstraint("*".to_string());
        assert!(c.is_satisfied_by(&Version("0.0.1".to_string())));
        assert!(c.is_satisfied_by(&Version("99.0.0".to_string())));
    }

    #[test]
    fn test_version_constraint_caret() {
        let c = VersionConstraint("^1.2.0".to_string());
        assert!(c.is_satisfied_by(&Version("1.2.0".to_string())));
        assert!(c.is_satisfied_by(&Version("1.9.9".to_string())));
        assert!(!c.is_satisfied_by(&Version("2.0.0".to_string())));
        assert!(!c.is_satisfied_by(&Version("1.1.0".to_string())));
    }

    #[test]
    fn test_version_constraint_tilde() {
        let c = VersionConstraint("~1.2.0".to_string());
        assert!(c.is_satisfied_by(&Version("1.2.0".to_string())));
        assert!(c.is_satisfied_by(&Version("1.2.9".to_string())));
        assert!(!c.is_satisfied_by(&Version("1.3.0".to_string())));
    }

    #[test]
    fn test_version_constraint_gte() {
        let c = VersionConstraint(">=1.0.0".to_string());
        assert!(c.is_satisfied_by(&Version("1.0.0".to_string())));
        assert!(c.is_satisfied_by(&Version("2.0.0".to_string())));
        assert!(!c.is_satisfied_by(&Version("0.9.0".to_string())));
    }

    #[test]
    fn test_version_constraint_lt() {
        let c = VersionConstraint("<2.0.0".to_string());
        assert!(c.is_satisfied_by(&Version("1.9.9".to_string())));
        assert!(!c.is_satisfied_by(&Version("2.0.0".to_string())));
    }

    #[test]
    fn test_version_constraint_composite() {
        let c = VersionConstraint(">=1.0.0, <2.0.0".to_string());
        assert!(c.is_satisfied_by(&Version("1.0.0".to_string())));
        assert!(c.is_satisfied_by(&Version("1.5.0".to_string())));
        assert!(!c.is_satisfied_by(&Version("2.0.0".to_string())));
        assert!(!c.is_satisfied_by(&Version("0.9.0".to_string())));
    }

    #[test]
    fn test_simple_resolution() {
        let mut data = HashMap::new();
        let pkg_name = PackageName("foo".to_string());
        data.insert(
            pkg_name.clone(),
            vec![PackageVersion {
                name: pkg_name.clone(),
                version: Version("1.0.0".to_string()),
                dependencies: vec![],
            }],
        );

        let registry = MockRegistry { data };
        let solver = PubGrubSolver::new(&registry);
        let root = vec![DependencyRequirement {
            name: PackageName("foo".to_string()),
            constraint: VersionConstraint("1.0.0".to_string()),
        }];

        let result = solver.resolve(root).unwrap();
        assert_eq!(
            result.get(&PackageName("foo".to_string())).unwrap().0,
            "1.0.0"
        );
    }

    #[test]
    fn test_conflict_resolution() {
        let mut data = HashMap::new();
        let pkg_name = PackageName("foo".to_string());
        data.insert(
            pkg_name.clone(),
            vec![PackageVersion {
                name: pkg_name.clone(),
                version: Version("1.0.0".to_string()),
                dependencies: vec![],
            }],
        );

        let registry = MockRegistry { data };
        let solver = PubGrubSolver::new(&registry);
        let root = vec![DependencyRequirement {
            name: PackageName("foo".to_string()),
            constraint: VersionConstraint("2.0.0".to_string()),
        }];

        let result = solver.resolve(root);
        assert!(result.is_err());
    }

    #[test]
    fn test_transitive_resolution() {
        let mut data = HashMap::new();
        let foo = PackageName("foo".to_string());
        let bar = PackageName("bar".to_string());

        data.insert(
            foo.clone(),
            vec![PackageVersion {
                name: foo.clone(),
                version: Version("1.0.0".to_string()),
                dependencies: vec![DependencyRequirement {
                    name: bar.clone(),
                    constraint: VersionConstraint("1.0.0".to_string()),
                }],
            }],
        );

        data.insert(
            bar.clone(),
            vec![PackageVersion {
                name: bar.clone(),
                version: Version("1.0.0".to_string()),
                dependencies: vec![],
            }],
        );

        let registry = MockRegistry { data };
        let solver = PubGrubSolver::new(&registry);
        let root = vec![DependencyRequirement {
            name: PackageName("foo".to_string()),
            constraint: VersionConstraint("1.0.0".to_string()),
        }];

        let result = solver.resolve(root).unwrap();
        assert_eq!(
            result.get(&PackageName("foo".to_string())).unwrap().0,
            "1.0.0"
        );
        assert_eq!(
            result.get(&PackageName("bar".to_string())).unwrap().0,
            "1.0.0"
        );
    }

    #[test]
    fn test_highest_version_selection() {
        let mut data = HashMap::new();
        let pkg = PackageName("lib".to_string());
        data.insert(
            pkg.clone(),
            vec![
                PackageVersion {
                    name: pkg.clone(),
                    version: Version("1.0.0".to_string()),
                    dependencies: vec![],
                },
                PackageVersion {
                    name: pkg.clone(),
                    version: Version("1.5.0".to_string()),
                    dependencies: vec![],
                },
                PackageVersion {
                    name: pkg.clone(),
                    version: Version("2.0.0".to_string()),
                    dependencies: vec![],
                },
            ],
        );

        let registry = MockRegistry { data };
        let solver = PubGrubSolver::new(&registry);
        let root = vec![DependencyRequirement {
            name: pkg.clone(),
            constraint: VersionConstraint("^1.0.0".to_string()),
        }];

        let result = solver.resolve(root).unwrap();
        // Should pick 1.5.0 (highest satisfying ^1.0.0, not 2.0.0)
        assert_eq!(result.get(&pkg).unwrap().0, "1.5.0");
    }

    #[test]
    fn test_lockfile_toml_output() {
        let mut data = HashMap::new();
        let foo = PackageName("foo".to_string());
        let bar = PackageName("bar".to_string());

        data.insert(
            foo.clone(),
            vec![PackageVersion {
                name: foo.clone(),
                version: Version("1.0.0".to_string()),
                dependencies: vec![DependencyRequirement {
                    name: bar.clone(),
                    constraint: VersionConstraint("^1.0.0".to_string()),
                }],
            }],
        );

        data.insert(
            bar.clone(),
            vec![PackageVersion {
                name: bar.clone(),
                version: Version("1.2.0".to_string()),
                dependencies: vec![],
            }],
        );

        let registry = MockRegistry { data };
        let solver = PubGrubSolver::new(&registry);
        let root = vec![DependencyRequirement {
            name: foo.clone(),
            constraint: VersionConstraint("^1.0.0".to_string()),
        }];

        let lockfile = solver.resolve_with_lock(root).unwrap();
        let toml = lockfile.to_toml();

        assert!(toml.contains("lock_version = 1"));
        assert!(toml.contains("foo"));
        assert!(toml.contains("bar"));
        assert!(toml.contains("name = \"foo\""));
        assert!(toml.contains("version = \"1.0.0\""));
    }

    #[test]
    fn test_lockfile_json_output() {
        let mut data = HashMap::new();
        let pkg = PackageName("json-test".to_string());
        data.insert(
            pkg.clone(),
            vec![PackageVersion {
                name: pkg.clone(),
                version: Version("1.0.0".to_string()),
                dependencies: vec![],
            }],
        );

        let registry = MockRegistry { data };
        let solver = PubGrubSolver::new(&registry);
        let root = vec![DependencyRequirement {
            name: pkg.clone(),
            constraint: VersionConstraint("1.0.0".to_string()),
        }];

        let lockfile = solver.resolve_with_lock(root).unwrap();
        let json = lockfile.to_json().unwrap();

        assert!(json.contains("json-test"));
        assert!(json.contains("1.0.0"));
        assert!(json.contains("lock_version"));
    }

    #[test]
    fn test_deterministic_lockfile() {
        let mut data = HashMap::new();
        let a = PackageName("a".to_string());
        let b = PackageName("b".to_string());
        let c = PackageName("c".to_string());

        data.insert(
            a.clone(),
            vec![PackageVersion {
                name: a.clone(),
                version: Version("1.0.0".to_string()),
                dependencies: vec![
                    DependencyRequirement {
                        name: b.clone(),
                        constraint: VersionConstraint("1.0.0".to_string()),
                    },
                    DependencyRequirement {
                        name: c.clone(),
                        constraint: VersionConstraint("1.0.0".to_string()),
                    },
                ],
            }],
        );

        for pkg in [&b, &c] {
            data.insert(
                pkg.clone(),
                vec![PackageVersion {
                    name: pkg.clone(),
                    version: Version("1.0.0".to_string()),
                    dependencies: vec![],
                }],
            );
        }

        let registry = MockRegistry { data };
        let solver = PubGrubSolver::new(&registry);
        let root = vec![DependencyRequirement {
            name: a.clone(),
            constraint: VersionConstraint("1.0.0".to_string()),
        }];

        // Resolve twice and check lockfiles are identical
        let lock1 = solver.resolve_with_lock(root.clone()).unwrap();
        let lock2 = solver.resolve_with_lock(root).unwrap();

        assert_eq!(lock1.to_toml(), lock2.to_toml());
        assert_eq!(lock1.to_json().unwrap(), lock2.to_json().unwrap());
    }

    #[test]
    fn test_package_not_found() {
        let registry = MockRegistry {
            data: HashMap::new(),
        };
        let solver = PubGrubSolver::new(&registry);
        let root = vec![DependencyRequirement {
            name: PackageName("nonexistent".to_string()),
            constraint: VersionConstraint("*".to_string()),
        }];

        let result = solver.resolve(root);
        assert!(result.is_err());
    }

    #[test]
    fn test_multi_version_with_constraints() {
        let mut data = HashMap::new();
        let lib = PackageName("lib".to_string());

        data.insert(
            lib.clone(),
            vec![
                PackageVersion {
                    name: lib.clone(),
                    version: Version("1.0.0".to_string()),
                    dependencies: vec![],
                },
                PackageVersion {
                    name: lib.clone(),
                    version: Version("1.1.0".to_string()),
                    dependencies: vec![],
                },
                PackageVersion {
                    name: lib.clone(),
                    version: Version("2.0.0".to_string()),
                    dependencies: vec![],
                },
            ],
        );

        let registry = MockRegistry { data };
        let solver = PubGrubSolver::new(&registry);

        // >=1.0.0, <2.0.0 should pick 1.1.0
        let root = vec![DependencyRequirement {
            name: lib.clone(),
            constraint: VersionConstraint(">=1.0.0, <2.0.0".to_string()),
        }];

        let result = solver.resolve(root).unwrap();
        assert_eq!(result.get(&lib).unwrap().0, "1.1.0");
    }
}
