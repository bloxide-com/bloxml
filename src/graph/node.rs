use core::fmt;

use petgraph::graph::NodeIndex;

pub(super) trait RustEntity: fmt::Debug {
    fn name(&self) -> String;
    fn full_path(&self) -> String;
}

#[derive(Debug, Clone)]
pub struct Crate {
    pub name: String,
    pub path: String,
}

impl Crate {
    pub fn new(name: String, path: String) -> Self {
        Self { name, path }
    }
}

impl RustEntity for Crate {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn full_path(&self) -> String {
        self.path.clone()
    }
}
#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub path: String,
}

impl Module {
    pub fn new(name: String, path: String) -> Self {
        Self { name, path }
    }
}

impl RustEntity for Module {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn full_path(&self) -> String {
        self.path.clone()
    }
}

#[derive(Debug, Clone)]
pub struct Type {
    pub name: String,
    pub path: String,
}

impl Type {
    pub fn new(name: String, path: String) -> Self {
        Self { name, path }
    }
}

impl RustEntity for Type {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn full_path(&self) -> String {
        self.path.clone()
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub path: String,
}

impl Function {
    pub fn new(name: String, path: String) -> Self {
        Self { name, path }
    }
}

impl RustEntity for Function {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn full_path(&self) -> String {
        self.path.clone()
    }
}

#[derive(Debug, Clone)]
pub struct Trait {
    pub name: String,
    pub path: String,
}

impl Trait {
    pub fn new(name: String, path: String) -> Self {
        Self { name, path }
    }
}

impl RustEntity for Trait {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn full_path(&self) -> String {
        self.path.clone()
    }
}

#[derive(Debug, Clone)]
pub enum Node {
    Crate(Crate),
    Module(Module),
    Type(Type),
    Function(Function),
    Trait(Trait),
}

impl RustEntity for Node {
    fn name(&self) -> String {
        match self {
            Node::Crate(krate) => krate.name.clone(),
            Node::Module(module) => module.name.clone(),
            Node::Type(typ) => typ.name.clone(),
            Node::Function(function) => function.name.clone(),
            Node::Trait(ktrait) => ktrait.name.clone(),
        }
    }

    fn full_path(&self) -> String {
        match self {
            Node::Crate(krate) => krate.path.clone(),
            Node::Module(module) => module.path.clone(),
            Node::Type(typ) => typ.path.clone(),
            Node::Function(function) => function.path.clone(),
            Node::Trait(trate) => trate.path.clone(),
        }
    }
}

impl Node {
    pub fn name(&self) -> String {
        match self {
            Node::Crate(name) => name.name(),
            Node::Module(name) => name.name(),
            Node::Type(name) => name.name(),
            Node::Function(name) => name.name(),
            Node::Trait(name) => name.name(),
        }
    }

    pub fn node_str(&self) -> &str {
        match self {
            Node::Crate(_) => "Crate",
            Node::Module(_) => "Module",
            Node::Type(_) => "Type",
            Node::Function(_) => "Function",
            Node::Trait(_) => "Trait",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Relation {
    Contains,
    Implements,
    Uses,
}

#[derive(Debug, Clone)]
pub struct Entry<'e> {
    pub index: NodeIndex,
    pub node: &'e Node,
}

impl<'e> Entry<'e> {
    pub fn new(index: NodeIndex, node: &'e Node) -> Self {
        Self { index, node }
    }
}

#[derive(Debug, Clone)]
pub struct RelatedEntry<'e> {
    pub index: NodeIndex,
    pub node: &'e Node,
    pub relation: Relation,
}

impl<'e> RelatedEntry<'e> {
    pub fn new(index: NodeIndex, node: &'e Node, relation: Relation) -> Self {
        Self {
            index,
            node,
            relation,
        }
    }

    pub fn relation(&self) -> Relation {
        self.relation
    }

    pub fn node(&self) -> &Node {
        self.node
    }

    pub fn index(&self) -> NodeIndex {
        self.index
    }
}
