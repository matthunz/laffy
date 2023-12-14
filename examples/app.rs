use laffy::LayoutTree;
use taffy::{geometry::Size, style::Style, style_helpers::TaffyMaxContent};

fn main() {
    let tree = LayoutTree::default();

    let node = tree.insert(Style {
        size: Size::from_points(100., 100.),
        ..Default::default()
    });

    node.measure(Size::MAX_CONTENT);

    dbg!(node.layout());
}
