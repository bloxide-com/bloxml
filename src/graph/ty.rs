use core::fmt;

/// Represents where a discovered type is located
#[derive(Debug, Clone, PartialEq)]
pub(super) enum TypeLocation {
    /// Built-in Rust type (no import needed)
    Builtin,
    /// Bloxide framework type with full import path
    BloxideFramework(String),
    /// Custom type defined in the actor
    ActorCustom(String),
    /// Unknown location (error case)
    Unknown,
}

/// Context about where a type was discovered
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(super) enum TypeContext {
    ExtendedState,
    Component,
    States,
    MessageSet,
    Runtime,
}

/// Information about a discovered type
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(super) struct DiscoveredType {
    pub name: String,
    pub full_type: String,
    pub used_in_module: String,
    pub context: TypeContext,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Import(String);

impl Import {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn rust_import(&self) -> String {
        format!("use {self};")
    }
}

impl From<String> for Import {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Import {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<Import> for String {
    fn from(value: Import) -> Self {
        value.0
    }
}

impl From<&Import> for String {
    fn from(value: &Import) -> Self {
        value.0.clone()
    }
}

impl fmt::Display for Import {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
