//! Control flow graph stuff

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

/// The data of a [`ControlFlowNode`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeRef(pub usize);

impl Display for NodeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
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
pub struct ControlFlowNode<Meta: Clone + Debug> {
    /// The associated data.
    pub data: NodeRef,
    /// The children of this node.
    pub children: HashSet<NodeRef>,
    /// The parents of this node.
    pub parents: HashSet<NodeRef>,
    /// Metadata for this node.
    pub meta: Meta,
}

/// A control flow graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlFlowGraph<Meta: Clone + Debug> {
    nodes: HashMap<NodeRef, ControlFlowNode<Meta>>,
    root: Option<NodeRef>,
}

impl<Meta: Clone + Debug> ControlFlowGraph<Meta> {
    /// Create a new control flow graph. The root is index 0.
    pub fn new(root: NodeRef, root_meta: Meta) -> Self {
        let mut new = Self::new_rootless();
        new.insert(NodeRef(0), root, root_meta);
        new
    }
    /// Create a new empty CFG. The root will be whatever is pushed first with a
    /// parent of 0.
    pub fn new_rootless() -> Self {
        Self {
            nodes: HashMap::new(),
            root: None,
        }
    }
    /// Insert a node without a parent.
    pub fn insert_parentless(&mut self, this: NodeRef, meta: Meta) {
        if self.nodes.is_empty() {
            eprintln!("_ -> {this}");
            self.nodes.insert(
                this,
                ControlFlowNode {
                    data: this,
                    children: HashSet::new(),
                    parents: HashSet::new(),
                    meta,
                },
            );
            self.root = Some(this);
            return;
        }
        eprintln!("_ -> {this}");
        match self.nodes.entry(this) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().meta = meta;
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(ControlFlowNode {
                    data: this,
                    children: HashSet::new(),
                    parents: HashSet::new(),
                    meta,
                });
            }
        }
    }
    /// Push a new node to the control flow graph with the provided parent.
    pub fn insert(&mut self, parent: NodeRef, this: NodeRef, meta: Meta) {
        if self.nodes.is_empty() {
            eprintln!("_ -> {this}");
            self.nodes.insert(
                this,
                ControlFlowNode {
                    data: this,
                    children: HashSet::new(),
                    parents: HashSet::new(),
                    meta,
                },
            );
            self.root = Some(this);
            return;
        }
        eprintln!("{parent} -> {this}");
        match self.nodes.entry(this) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().parents.insert(parent);
                entry.get_mut().meta = meta;
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(ControlFlowNode {
                    data: this,
                    children: HashSet::new(),
                    parents: [parent].into(),
                    meta,
                });
            }
        }

        self.nodes.get_mut(&parent).unwrap().children.insert(this);
    }
    /// Remove the provided item from the CFG.
    pub fn remove(&mut self, this: NodeRef) {
        let node = self.nodes.remove(&this).unwrap();
        for child in node.children {
            self.nodes.get_mut(&child).unwrap().parents.remove(&this);
        }
        for parent in node.parents {
            self.nodes.get_mut(&parent).unwrap().children.remove(&this);
        }
    }
    /// Get the children of a node.
    pub fn children_of(&self, node: NodeRef) -> &HashSet<NodeRef> {
        &self.nodes[&node].children
    }
    /// Get the parents of a node.
    pub fn parents_of(&self, node: NodeRef) -> &HashSet<NodeRef> {
        &self.nodes[&node].parents
    }
    /// Get the meta of a node.
    pub fn meta_of(&self, node: NodeRef) -> &Meta {
        &self.nodes[&node].meta
    }
    /// Whether this CFG contains the provided data item.
    pub fn has(&self, data: NodeRef) -> bool {
        self.nodes.contains_key(&data)
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
            keys: self.nodes.keys(),
        }
    }
    /// Get the number of nodes in this graph.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

type Ed = (NodeRef, NodeRef);

impl<'a, Meta: Clone + Debug> dot::Labeller<'a, NodeRef, Ed> for ControlFlowGraph<Meta> {
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

impl<'a, Meta: Clone + Debug> dot::GraphWalk<'a, NodeRef, Ed> for ControlFlowGraph<Meta> {
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

struct ControlFlowGraphIter<'a, Meta: Clone + Debug> {
    keys: std::collections::hash_map::Keys<'a, NodeRef, ControlFlowNode<Meta>>,
}

impl<'a, Meta: Clone + Debug> Iterator for ControlFlowGraphIter<'a, Meta> {
    type Item = NodeRef;

    fn next(&mut self) -> Option<Self::Item> {
        self.keys.next().copied()
    }
}
