#![no_std]
#![no_main]

use aya_ebpf::{bindings::xdp_action, macros::xdp, programs::XdpContext};
use aya_log_ebpf::debug;
use aya_metrics_ebpf::counter;

use aya_metrics_example_common::MyCounter;

#[xdp]
pub fn aya_metrics_example(ctx: XdpContext) -> u32 {
    match try_aya_metrics_example(ctx) {
        Ok(ret) => ret,
        Err(_) => xdp_action::XDP_ABORTED,
    }
}

fn try_aya_metrics_example(ctx: XdpContext) -> Result<u32, u32> {
    debug!(&ctx, "received a packet");
    counter(MyCounter::Packets, 1);
    Ok(xdp_action::XDP_PASS)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
