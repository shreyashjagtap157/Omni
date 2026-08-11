use omni_compiler::package::solver::*;
use std::collections::HashMap;

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
