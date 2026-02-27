//! Control flow graph stuff

use std::{
    borrow::Cow,
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
    pub parents: HashSet<NodeRef>,
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
        new.insert(NodeRef(0), root);
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
    /// If the provided parent doesn't exist in this graph, panics. An exception
    /// is if this CFG is empty.
    pub fn insert(&mut self, parent: NodeRef, this: NodeRef) {
        if self.nodes.is_empty() {
            eprintln!("_ -> {this}");
            self.nodes.insert(
                this,
                ControlFlowNode {
                    data: this,
                    children: HashSet::new(),
                    parents: HashSet::new(),
                },
            );
            self.set.insert(this);
            self.root = Some(this);
            return;
        }
        eprintln!("{parent} -> {this}");
        assert!(*parent < self.nodes.len());
        match self.nodes.entry(this) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().parents.insert(parent);
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(ControlFlowNode {
                    data: this,
                    children: HashSet::new(),
                    parents: [parent].into(),
                });
            }
        }

        self.nodes.get_mut(&parent).unwrap().children.insert(this);
        self.set.insert(this);
    }
    /// Get the children of a node.
    pub fn children_of(&self, node: NodeRef) -> &HashSet<NodeRef> {
        &self.nodes[&node].children
    }
    /// Get the parents of a node.
    pub fn parents_of(&self, node: NodeRef) -> &HashSet<NodeRef> {
        &self.nodes[&node].parents
    }
    /// Whether this CFG contains the provided data item.
    pub fn has(&self, data: NodeRef) -> bool {
        self.set.contains(&data)
    }
    /// Convert this CFG into a .dot graphviz file.
    pub fn to_dot(&self) -> String {
        let mut out = Vec::<u8>::new();

        dot::render(self, &mut out).unwrap();

        out.try_into().unwrap()
    }
    /// Get an iterator over the items in an arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = NodeRef> {
        ControlFlowGraphIter {
            hash_set_iter: self.set.iter(),
        }
    }
    /// Get the number of nodes in this graph.
    pub fn len(&self) -> usize {
        self.set.len()
    }
}

type Ed = (NodeRef, NodeRef);

impl<'a> dot::Labeller<'a, NodeRef, Ed> for ControlFlowGraph {
    fn graph_id(&'a self) -> dot::Id<'a> {
        dot::Id::new("cfg1").unwrap()
    }

    fn node_id(&'a self, n: &NodeRef) -> dot::Id<'a> {
        dot::Id::new(format!("N{}", *n)).unwrap()
    }

    fn node_label(&'a self, n: &NodeRef) -> dot::LabelText<'a> {
        dot::LabelText::LabelStr(n.to_string().into())
    }

    fn node_shape(&'a self, node: &NodeRef) -> Option<dot::LabelText<'a>> {
        match self.children_of(*node).len() {
            0 => Some(dot::LabelText::LabelStr("box".into())),
            1 => None,
            _ => Some(dot::LabelText::LabelStr("diamond".into())),
        }
    }
}

impl<'a> dot::GraphWalk<'a, NodeRef, Ed> for ControlFlowGraph {
    fn nodes(&self) -> dot::Nodes<'a, NodeRef> {
        Cow::Owned(self.nodes.values().map(|v| v.data).collect::<Vec<_>>())
    }

    fn edges(&'a self) -> dot::Edges<'a, Ed> {
        Cow::Owned(
            self.nodes
                .values()
                .flat_map(|v| v.children.iter().map(|v2| (v.data, *v2)))
                .collect::<Vec<_>>(),
        )
    }

    fn source(&self, e: &Ed) -> NodeRef {
        e.0
    }

    fn target(&self, e: &Ed) -> NodeRef {
        e.1
    }
}

struct ControlFlowGraphIter<'a> {
    hash_set_iter: std::collections::hash_set::Iter<'a, NodeRef>,
}

impl<'a> Iterator for ControlFlowGraphIter<'a> {
    type Item = NodeRef;

    fn next(&mut self) -> Option<Self::Item> {
        self.hash_set_iter.next().copied()
    }
}
