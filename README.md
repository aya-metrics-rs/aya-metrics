# aya-metrics - a metrics library for eBPF programs

## Overview

`aya-metrics` is a metrics library for eBPF programs written using [aya]. Think of it as the [metrics] crate for eBPF.

## Installation

### User space

Add `aya-metrics` and `aya-metrics-common`:

```console
cargo add aya-metrics
cargo add aya-metrics-common --features user
```

If your user space and eBPF crates share a common crate you may consider adding `aya-metrics-common`:

```console
cargo add aya-metrics-common
```

### eBPF side

Add `aya-metrics-ebpf`:

```console
cargo add aya-metrics-ebpf
```

## Example

Here's an example that uses `aya-metrics` in conjunction with the [metrics_printer] crate to publish eBPF metrics your console.

### Common code
```rust
// #![no_std] except for tests and user space!
#![cfg_attr(not(any(test, feature = "user")), no_std)]

#[derive(Copy, Clone)]
pub enum MyCounter {
    Packets,
    Bytes,
}

impl aya_metrics_common::Counter for MyCounter {
    #[cfg(any(test, feature = "user"))]
    fn name(self) -> String {
        match self {
            MyCounter::Packets => "packets_counter".to_string(),
            MyCounter::Bytes => "bytes_counter".to_string(),
        }
    }

    fn index(&self) -> u32 {
        match self {
            MyCounter::Packets => 0,
            MyCounter::Bytes => 1,
        }
    }
}
```

### User space code

```rust
use std::time::Duration;
use aya_metrics::{EbpfMetrics, Dimension, Metric};
use metrics::{Label, Unit};
use metrics_printer::PrintRecorder;

use my_common::MyCounter;

let metrics = vec![
    Metric::new(
        MyCounter::Packets,
        Unit::Count,
        vec![
            Dimension::By(vec![]),
            Dimension::By(vec![Label::new("hostname", "test.hostname")]),
        ]
    )
];


PrintRecorder::default().install().unwrap();
if let Err(e) = EbpfMetrics::new(&mut ebpf, metrics, Duration::from_secs(1)).map(|m| tokio::spawn(m.run())) {
    warn!("failed to initialize eBPF metrics: {}", e);
}
```

### eBPF code

```rust
use aya_ebpf::{bindings::xdp_action, macros::xdp, programs::XdpContext};
use aya_metrics_ebpf::counter;

use my_common::MyCounter;

#[xdp]
pub fn xdp_prog(ctx: XdpContext) -> u32 {
    counter(MyCounter::Packets, 1);
    xdp_action::XDP_PASS
}
```

## 🚧 TODO
Any help is welcome!

🚧 Move from using user/~~bpf~~ features to using target architecture

❌ Support a custom number of counters (currently hard coded to 64)

❌ Support multiple (custom named) counter maps

❌ Release to crates.io 🎉

❌ Deprecate aya-metrics-mocks crate in favour of mocking support in aya



[aya]: https://github.com/aya-rs/aya
[metrics]: https://docs.rs/metrics
[metrics_cloudwatch]: https://docs.rs/metrics_cloudwatch
