//! Control flow graph stuff

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    ops::{Deref, DerefMut},
};

/// The data of a [`ControlFlowNode`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeRef(pub usize);

impl Display for NodeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for NodeRef {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for NodeRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A single node in a [`ControlFlowGraph`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlFlowNode {
    /// The associated data.
    pub data: NodeRef,
    /// The children of this node.
    pub children: HashSet<NodeRef>,
    /// The parents of this node.
    pub parent: HashSet<NodeRef>,
}

/// A control flow graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlFlowGraph {
    nodes: HashMap<NodeRef, ControlFlowNode>,
    set: HashSet<NodeRef>,
    root: Option<NodeRef>,
}

impl ControlFlowGraph {
    /// Create a new control flow graph. The root is index 0.
    pub fn new(root: NodeRef) -> Self {
        let mut new = Self::new_rootless();
        new.push(NodeRef(0), root);
        new
    }
    /// Create a new empty CFG. The root will be whatever is pushed first with a
    /// parent of 0.
    pub fn new_rootless() -> Self {
        Self {
            nodes: HashMap::new(),
            set: HashSet::new(),
            root: None,
        }
    }
    /// Push a new node to the control flow graph with the provided parent.
    ///
    /// # Panics
    ///
    /// If the provided index doesn't exist in this graph, panics. An exception
    /// is if this CFG is empty and the index provided is 0.
    pub fn push(&mut self, parent: NodeRef, data: NodeRef) {
        println!("({parent}, {data})");

        if *parent == 0 && self.nodes.is_empty() {
            self.nodes.insert(
                data,
                ControlFlowNode {
                    data,
                    children: HashSet::new(),
                    parent: HashSet::new(),
                },
            );
            self.set.insert(data);
            self.root = Some(data);
        }
        assert!(*parent < self.nodes.len());
        match self.nodes.entry(data) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().parent.insert(parent);
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(ControlFlowNode {
                    data,
                    children: HashSet::new(),
                    parent: [parent].into(),
                });
            }
        }

        self.nodes.get_mut(&parent).unwrap().children.insert(data);
        self.set.insert(data);
    }
    /// Get the children of a node.
    pub fn children_of(&self, node: NodeRef) -> &HashSet<NodeRef> {
        &self.nodes[&node].children
    }
    /// Get the data associated with a node.
    pub fn data_of(&self, node: NodeRef) -> NodeRef {
        self.nodes[&node].data
    }
    /// Whether this CFG contains the provided data item.
    pub fn has(&self, data: NodeRef) -> bool {
        self.set.contains(&data)
    }
}
