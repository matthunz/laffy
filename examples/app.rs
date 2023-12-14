use laffy::Tree;
use taffy::{geometry::Size, style_helpers::TaffyMaxContent};

#[tokio::main]
async fn main() {
    let tree = Tree::new();

    let parent = tree.node();

    let a = tree.node();
    parent.add_child(a.clone());

    let b = tree.node();
    parent.add_child(b);

    parent.measure(Size::MAX_CONTENT).await;

    dbg!(a.layout());
}
