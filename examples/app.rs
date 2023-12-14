use laffy::Tree;
use taffy::{geometry::Size, style_helpers::TaffyMaxContent};

#[tokio::main]
async fn main() {
    let tree = Tree::new();

    let parent = tree.node();

    let a = tree.node();
    let _b = tree.node();

    parent.measure(Size::MAX_CONTENT).await;

    dbg!(a.layout());
}
