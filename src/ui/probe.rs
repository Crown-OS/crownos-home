//! Follows the size of the surface into a signal the board lays out against.

use crownui::{
    ext::{Kind, NodeId},
    prelude::{Cx, Element, Signal, Size},
};

pub struct SizeProbeKind {
    target: Signal<Size>,
}

impl Kind for SizeProbeKind {
    const NAME: &'static str = "size_probe";

    fn mount(self, cx: &mut Cx, node: NodeId) {
        let observed = cx.observe_size(node);
        let target = self.target;
        cx.effect(move |runtime| {
            let size = observed.get(runtime);
            if target.get(runtime) != size {
                target.set(runtime, size);
            }
        });
    }
}

pub fn size_probe(target: Signal<Size>) -> Element<SizeProbeKind> {
    Element::new(SizeProbeKind { target })
}
