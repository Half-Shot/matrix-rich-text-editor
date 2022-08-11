use wysiwyg::{ComposerModel};

fn main() {
    let size = 10_000;
    let mut model = ComposerModel::new();

    let to_add: [u16; 1] = [12];
    for _ in 0..size {
        model.replace_text(&to_add);
    }
}
