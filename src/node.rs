use crate::{cursor::Cursor, document::Document, util::mutstr::MutStr};
use serde::Serialize;
use sha2::{Digest, Sha512Trunc256};
use tracing::trace;

#[derive(Debug, Serialize, Clone)]
pub enum Action {
    Null,
    Insert {
        offset: usize,
        body: String,
    },
    #[allow(unused)]
    Delete {
        offset: usize,
    },
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct NodeId(pub [u8; 32]);

#[derive(Debug, Clone)]
pub struct Node {
    pub tick: u32,
    pub parent: Option<NodeId>,
    pub action: Action,
}

impl NodeId {
    pub fn hex4(&self) -> String {
        hex::encode(&self.0[0..2])
    }
}

static NULL: &'static [u8; 32] = &[0; 32];

impl Node {
    #[allow(unused)]
    pub fn new(tick: u32, parent: Option<NodeId>, action: Action) -> Self {
        Node {
            tick,
            action,
            parent: parent,
        }
    }
    pub fn root(tick: u32) -> Node {
        Node {
            tick,
            parent: None,
            action: Action::Null,
        }
    }
    pub fn insert(cursor: &Cursor, body: String) -> Self {
        let tick = cursor.doc().increment_clock();

        Node {
            tick,
            action: Action::Insert {
                offset: cursor.offset,
                body,
            },
            parent: Some(cursor.node_id.clone()),
        }
    }
    pub fn delete(cursor: &Cursor) -> Self {
        let tick = cursor.doc().increment_clock();

        Node {
            tick,
            action: Action::Delete {
                offset: cursor.offset,
            },
            parent: Some(cursor.node_id.clone()),
        }
    }
    pub fn parent(&self) -> Option<&NodeId> {
        self.parent.as_ref()
    }
    pub fn parent_hex4(&self) -> String {
        if let Some(p) = &self.parent {
            p.hex4()
        } else {
            "NA".to_string()
        }
    }
    pub fn diag(&self) -> String {
        use crate::node::Action::*;
        match &self.action {
            Null => "NULL".to_string(),
            Action::Insert { offset, body } => format!("{} @ {}", body, offset),
            Action::Delete { offset } => format!("␡ @ {}", offset),
        }
    }
    pub fn offset(&self) -> usize {
        use crate::node::Action::*;
        match &self.action {
            Null => 0,
            Insert { offset, .. } | Delete { offset, .. } => *offset,
        }
    }
    #[allow(unused)]
    pub fn node_id(&self) -> NodeId {
        let mut hasher = Sha512Trunc256::new();
        if let Some(parent) = self.parent() {
            hasher.update(parent.0);
        } else {
            hasher.update(NULL);
        }
        serde_json::to_writer(&mut hasher, &self.tick);
        serde_json::to_writer(&mut hasher, &self.action);

        // read hash digest and consume hasher
        let result: [u8; 32] = hasher.finalize().into();

        NodeId(result)
    }
    pub fn find(&self, c: char) -> Option<usize> {
        match &self.action {
            Action::Insert { offset, body } => body.find(c),
            _ => None,
        }
    }
    pub fn project(&self, buf: &mut MutStr, limit: Option<usize>) -> isize {
        match &self.action {
            Action::Null => {
                trace!("{}: root", self.node_id().hex4());
                0
            }
            Action::Insert { offset, body } => {
                trace!(
                    "{}: insert({} of {}, {}) ({})",
                    self.node_id().hex4(),
                    offset,
                    buf.len(),
                    body,
                    self.parent_hex4()
                );

                let slice = match limit {
                    Some(limit) => &body[0..body.len().min(limit)],
                    None => &body[..],
                };

                // TODO calculate render offset here
                buf.insert_str(*offset, &slice);
                slice.len() as isize
            }
            Action::Delete { offset } => {
                trace!(
                    "{}: delete({}) ({})",
                    self.node_id().hex4(),
                    offset,
                    self.parent_hex4()
                );
                buf.remove(*offset);

                -1
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Action, Node};

    #[test]
    fn test1() {
        let node0 = Node::root(0);
        let node1 = Node::new(
            1,
            Some(node0.node_id()),
            Action::Insert {
                offset: 0,
                body: "H".to_string(),
            },
        );

        let node2 = Node::new(
            2,
            Some(node1.node_id()),
            Action::Insert {
                offset: 0,
                body: "e".to_string(),
            },
        );

        let foo: &[u8; 32] = &(node2.node_id().0);
        assert_eq!(
            hex::encode(foo),
            "37dbcb6c5f48e99e4530ab2b4b76731abacdca9a3e93dba49690cdbbd69d90b1"
        )
    }
}
