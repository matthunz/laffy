use slotmap::DefaultKey;
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::{Rc, Weak},
};
use taffy::{
    geometry::Size,
    layout::Layout,
    style::{AvailableSpace, Style},
    Taffy,
};

#[derive(Default, Clone)]
pub struct LayoutTree {
    taffy: Rc<RefCell<Taffy>>,
}

impl LayoutTree {
    pub fn insert(&self, style: Style) -> Rc<Node> {
        let key = self.taffy.borrow_mut().new_leaf(style).unwrap();
        let node = Node {
            key,
            parent: RefCell::default(),
            children: RefCell::default(),
            tree: self.clone(),
            layout: RefCell::new(Layout::new()),
        };
        Rc::new(node)
    }
}

pub struct Node {
    key: DefaultKey,
    parent: RefCell<Option<Weak<Self>>>,
    children: RefCell<Vec<Rc<Self>>>,
    // TODO weak?
    tree: LayoutTree,
    layout: RefCell<Layout>,
}

impl Node {
    pub fn parent(&self) -> Option<Rc<Self>> {
        self.parent
            .borrow()
            .as_ref()
            .map(|parent| parent.upgrade().unwrap())
    }

    pub fn children(&self) -> Vec<Rc<Self>> {
        self.children.borrow().clone()
    }

    pub fn add_child(&self, child: Rc<Self>) {
        let key = child.key;
        self.children.borrow_mut().push(child);
        self.tree
            .taffy
            .borrow_mut()
            .add_child(self.key, key)
            .unwrap();
    }

    pub fn remove_child(&self, index: usize) -> Rc<Self> {
        let child = self.children.borrow_mut().remove(index);
        self.tree
            .taffy
            .borrow_mut()
            .remove_child(self.key, child.key)
            .unwrap();
        child
    }

    pub fn measure(&self, available_space: Size<AvailableSpace>) {
        let mut taffy = self.tree.taffy.borrow_mut();
        taffy.compute_layout(self.key, available_space).unwrap();

        enum Item {
            Push(Rc<Node>),
            Pop,
        }

        let mut stack: Vec<_> = self
            .children
            .borrow()
            .iter()
            .map(|child| Item::Push(child.clone()))
            .collect();

        let mut layouts: Vec<Layout> = Vec::new();

        let mut layout = *taffy.layout(self.key).unwrap();
        if let Some(parent_layout) = layouts.last() {
            layout.location.x += parent_layout.location.x;
            layout.location.x += parent_layout.location.x;
        }

        *self.layout.borrow_mut() = layout;
        layouts.push(layout);

        while let Some(item) = stack.pop() {
            match item {
                Item::Push(node) => {
                    let mut layout = *taffy.layout(node.key).unwrap();
                    if let Some(parent_layout) = layouts.last() {
                        layout.location.x += parent_layout.location.x;
                        layout.location.x += parent_layout.location.x;
                    }

                    layouts.push(layout);
                    *node.layout.borrow_mut() = layout;

                    stack.push(Item::Pop);
                    stack.extend(
                        node.children
                            .borrow()
                            .iter()
                            .map(|child| Item::Push(child.clone())),
                    )
                }
                Item::Pop => {
                    layouts.pop();
                }
            }
        }
    }

    pub fn layout(&self) -> Layout {
        *self.layout.borrow()
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        self.tree.taffy.borrow_mut().remove(self.key).unwrap();

        for child in &*self.children.borrow() {
            *child.parent.borrow_mut() = None;
        }
    }
}
