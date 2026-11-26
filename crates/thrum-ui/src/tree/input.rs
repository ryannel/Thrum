use crate::{Button, InputLayout, NodeId, NodeKind, Scope, Tree, input::Control};

impl Tree {
    /// Policy of a declared scope that the game can open.
    pub fn scope(&self, id: NodeId) -> Option<Scope> {
        match self.nodes[*self.ids.get(&id)?].node.kind {
            NodeKind::Scope(policy) => Some(policy),
            _ => None,
        }
    }

    /// Change a button's eligibility without changing its geometry.
    pub fn set_enabled(&mut self, id: NodeId, enabled: bool) {
        if let Some(&index) = self.ids.get(&id)
            && let NodeKind::Button(Button { enabled: value, .. }) =
                &mut self.nodes[index].node.kind
        {
            *value = enabled;
        }
    }

    /// Snapshot input geometry after laying out the tree for a paint list.
    pub fn input_layout(&self) -> InputLayout {
        let mut layout = InputLayout::default();
        for entry in self.nodes.iter().filter(|entry| entry.visible) {
            let id = entry.node.id;
            match entry.node.kind {
                NodeKind::Button(button) => layout.controls.push(Control {
                    id,
                    bounds: entry.rect,
                    button,
                    scope: self.parent_scope(entry.parent),
                }),
                NodeKind::DropTarget(limit) => {
                    layout
                        .drops
                        .push((self.parent_scope(entry.parent), id, limit));
                }
                _ => {}
            }
        }
        layout
    }

    fn parent_scope(&self, mut parent: Option<usize>) -> Option<NodeId> {
        while let Some(index) = parent {
            let entry = &self.nodes[index];
            if matches!(entry.node.kind, NodeKind::Scope(_)) {
                return Some(entry.node.id);
            }
            parent = entry.parent;
        }
        None
    }
}
