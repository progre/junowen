use junowen_lib::{Fn10f720, Th19};
use tracing::trace;

pub fn on_rewrite_controller_assignments(th19: &mut Th19, old_fn: fn(&mut Th19) -> Fn10f720) {
    let input_devices = th19.input_devices_mut();
    let old_p1_idx = input_devices.p1_idx();
    trace!(
        "on_rewrite_controller_assignments: before old_p1_idx={}",
        old_p1_idx
    );
    old_fn(th19)();
    if old_p1_idx == 0 && input_devices.p1_idx() != 0 {
        trace!(
            "on_rewrite_controller_assignments: after input_devices.p1_idx()={}",
            input_devices.p1_idx()
        );
        input_devices.set_p1_idx(0);
        trace!(
            "on_rewrite_controller_assignments: fixed input_devices.p1_idx()={}",
            input_devices.p1_idx()
        );
    }
}
