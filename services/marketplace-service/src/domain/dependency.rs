use crate::models::package::{DependencyRequirement, PackageVersion};

pub fn resolve_dependencies(version: &PackageVersion) -> Vec<DependencyRequirement> {
    version.dependencies.clone()
}
