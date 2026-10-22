//! The server's OAuth scope and its one permission. The installation grants
//! `console.use` to every member; the Hub decides each call the tools make for the
//! caller. Both spellings derive from [`crate::SLUG`].
use std::sync::LazyLock;

use veoveo_hub_sdk::registration::{Access, PermissionDefinition};
use veoveo_types::{PermissionName, ScopeDefinition, ScopeName, Vocabulary};

/// The OAuth scope that names the server's namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConsoleScope {
    Console,
}

impl ConsoleScope {
    pub const ALL: &'static [Self] = &[Self::Console];
}

impl ScopeDefinition for ConsoleScope {
    fn name(self) -> &'static ScopeName {
        static NAME: LazyLock<ScopeName> =
            LazyLock::new(|| ScopeName::parse(crate::SLUG).expect("the slug is a scope token"));
        &NAME
    }
}

/// The server's permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConsolePermission {
    /// Use the console's tools; the Hub decides each call for the caller.
    Use,
}

impl Vocabulary for ConsolePermission {
    const ALL: &'static [Self] = &[Self::Use];
    const NAME: &'static str = "ConsolePermission";

    fn as_str(self) -> &'static str {
        static USE: LazyLock<String> = LazyLock::new(|| format!("{}.use", crate::SLUG));
        match self {
            Self::Use => USE.as_str(),
        }
    }
}

impl ConsolePermission {
    /// The permission's wire name, as declared and as granted.
    pub fn name(self) -> PermissionName {
        PermissionName::parse(self.as_str()).expect("declared console permission spelling")
    }
}

impl PermissionDefinition for ConsolePermission {
    fn resource(self) -> &'static str {
        static ACTIVITY: LazyLock<String> = LazyLock::new(|| format!("{}.activity", crate::SLUG));
        ACTIVITY.as_str()
    }

    /// The tools change the caller's selection and watches.
    fn access(self) -> Access {
        Access::Write
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_permission_and_scope_derive_from_the_slug() {
        assert_eq!(ConsolePermission::Use.name().as_str(), "console.use");
        assert!(ConsolePermission::Use.name().is_owned_by(&crate::server()));
        assert_eq!(ConsolePermission::Use.resource(), "console.activity");
        assert_eq!(ConsoleScope::Console.name().as_str(), "console");
    }
}
