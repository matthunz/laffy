use slotmap::DefaultKey;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use taffy::{style::Style, Taffy};

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
}

impl Drop for Node {
    fn drop(&mut self) {
        self.tree.taffy.borrow_mut().remove(self.key).unwrap();

        for child in &*self.children.borrow() {
            *child.parent.borrow_mut() = None;
        }
    }
}
