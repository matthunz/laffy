use slotmap::{DefaultKey, SlotMap, SparseSecondaryMap};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    thread,
};
use taffy::{
    geometry::Size,
    layout::Layout,
    style::{AvailableSpace, Style},
    Taffy,
};
use tokio::sync::{mpsc, oneshot};

struct NodeData {
    parent: Option<Weak<Node>>,
    children: Vec<NodeRef>,
    layout: Layout,
}

pub struct Node {
    key: DefaultKey,
    tree: Tree,
    data: RefCell<NodeData>,
}

#[derive(Clone)]
pub struct NodeRef {
    node: Rc<Node>,
}

impl NodeRef {
    pub fn layout(&self) -> Layout {
        self.node.data.borrow().layout
    }

    pub fn add_child(&self, child: Self) {
        self.node
            .tree
            .tx
            .send(Request::AddChild {
                parent_key: self.node.key,
                child_key: child.node.key,
            })
            .unwrap();

        child.node.data.borrow_mut().parent = Some(Rc::downgrade(&self.node));
        self.node.data.borrow_mut().children.push(child);
    }

    pub async fn measure(&self, available_space: Size<AvailableSpace>) {
        let (tx, rx) = oneshot::channel();
        self.node
            .tree
            .tx
            .send(Request::Layout {
                key: self.node.key,
                available_space,
                tx,
            })
            .unwrap();
        let changes = rx.await.unwrap();

        let tree = self.node.tree.inner.borrow_mut();
        for (key, layout) in changes {
            tree.nodes[key].data.borrow_mut().layout = layout;
        }
    }
}

enum Request {
    Insert {
        key: DefaultKey,
        style: Style,
    },
    AddChild {
        parent_key: DefaultKey,
        child_key: DefaultKey,
    },
    Layout {
        key: DefaultKey,
        available_space: Size<AvailableSpace>,
        tx: oneshot::Sender<Vec<(DefaultKey, Layout)>>,
    },
}

struct Inner {
    nodes: SlotMap<DefaultKey, Rc<Node>>,
}

#[derive(Clone)]
pub struct Tree {
    tx: mpsc::UnboundedSender<Request>,
    inner: Rc<RefCell<Inner>>,
}

struct Data {
    layout_key: DefaultKey,
    layout: Layout,
}

impl Tree {
    pub fn new() -> Self {
        let (req_tx, mut req_rx) = mpsc::unbounded_channel();

        thread::spawn(move || {
            let mut taffy = Taffy::new();
            let mut nodes = SparseSecondaryMap::new();
            while let Some(req) = req_rx.blocking_recv() {
                match req {
                    Request::Insert { key, style } => {
                        let layout_key = taffy.new_leaf(style).unwrap();
                        nodes.insert(
                            key,
                            Data {
                                layout_key,
                                layout: Layout::new(),
                            },
                        );
                    }
                    Request::AddChild {
                        parent_key,
                        child_key,
                    } => {
                        let parent_layout_key = nodes[parent_key].layout_key;
                        let child_layout_key = nodes[child_key].layout_key;
                        taffy
                            .add_child(parent_layout_key, child_layout_key)
                            .unwrap();
                    }
                    Request::Layout {
                        key,
                        available_space,
                        tx,
                    } => {
                        taffy
                            .compute_layout(nodes[key].layout_key, available_space)
                            .unwrap();

                        enum Item {
                            Push(DefaultKey),
                            Pop,
                        }

                        let mut stack = vec![Item::Push(key)];
                        let mut layouts: Vec<Layout> = Vec::new();
                        let mut changes = Vec::new();

                        while let Some(item) = stack.pop() {
                            match item {
                                Item::Push(key) => {
                                    let mut layout = *taffy.layout(key).unwrap();
                                    if let Some(parent_layout) = layouts.last() {
                                        layout.location.x += parent_layout.location.x;
                                        layout.location.x += parent_layout.location.x;
                                    }

                                    layouts.push(layout);

                                    let last_layout = &mut nodes[key].layout;
                                    if last_layout.location != layout.location
                                        || last_layout.size != layout.size
                                        || last_layout.order != layout.order
                                    {
                                        changes.push((key, layout));
                                        nodes[key].layout = layout;
                                    }

                                    let data = &nodes[key];
                                    stack.push(Item::Pop);
                                    stack.extend(
                                        taffy
                                            .children(data.layout_key)
                                            .unwrap()
                                            .iter()
                                            .map(|child| Item::Push(child.clone())),
                                    )
                                }
                                Item::Pop => {
                                    layouts.pop();
                                }
                            }
                        }

                        if !changes.is_empty() {
                            tx.send(changes).unwrap();
                        }
                    }
                }
            }
        });

        Self {
            tx: req_tx,
            inner: Rc::new(RefCell::new(Inner {
                nodes: SlotMap::new(),
            })),
        }
    }

    pub fn node(&self) -> NodeRef {
        let mut cell = None;
        let key = self.inner.borrow_mut().nodes.insert_with_key(|key| {
            let node = Rc::new(Node {
                key,
                tree: self.clone(),
                data: RefCell::new(NodeData {
                    parent: None,
                    layout: Layout::new(),
                    children: Vec::new(),
                }),
            });
            cell = Some(node.clone());
            node
        });

        self.tx
            .send(Request::Insert {
                style: Style {
                    size: Size::from_points(100., 100.),
                    ..Default::default()
                },
                key,
            })
            .unwrap();

        NodeRef {
            node: cell.unwrap(),
        }
    }
}
