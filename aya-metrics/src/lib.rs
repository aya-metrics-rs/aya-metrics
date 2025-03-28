//! Gets metrics from an eBPF program!
//!
//! This is a generalized user space implementation to collect custom metrics from an eBPF program.
//!
//! The module provides the [EbpfMetrics] type, which reads counters created in eBPF and emits them using the [metrics]
//! crate. Any implementation of the [metrics::recorder::Recorder] trait can be used once it is set as the global recorder.
//!
//! # Example:
//!
//! Define counters:
//!
//! ```
//! # use std::time::Duration;
//! # use aya_metrics::{EbpfMetrics, Dimension, Metric};
//! # use metrics::{Label, Unit};
//!
//! #[derive(Copy, Clone)]
//! enum MyCounter {
//!     Packets,
//!     Bytes,
//! }
//!
//! impl aya_metrics_common::Counter for MyCounter {
//!     fn name(self) -> String {
//!         match self {
//!             MyCounter::Packets => "packets_counter".to_string(),
//!             MyCounter::Bytes => "bytes_counter".to_string(),
//!         }
//!     }
//!
//!     fn index(&self) -> u32 {
//!         match self {
//!             MyCounter::Packets => 0,
//!             MyCounter::Bytes => 1,
//!         }
//!     }
//! }
//!
//! let metrics = vec![
//!     Metric::new(
//!         MyCounter::Packets,
//!         Unit::Count,
//!         vec![
//!             Dimension::By(vec![]),
//!             Dimension::By(vec![Label::new("hostname", "test.hostname")]),
//!         ]
//!     )
//! ];
//! ```
//!
//! Emit metrics:
//!
//! ```ignore
//! # let mut bpf = aya::Ebpf::load(&[]).unwrap();
//! // start emitting metrics using the global recorder
//! EbpfMetrics::new(&mut bpf, metrics, Duration::from_secs(60)).unwrap();
//! ```
//!
//! With the following eBPF code:
//!
//! ```ignore
//! use aya_metrics_common::metrics::{counter, Counter};
//! use my_crate::MyCounter;
//!
//! counter(MyCounter::Packets, 1);
//! ```
//!
use std::{io, sync::Arc};

#[cfg(not(feature = "mocks"))]
use aya::Ebpf;
use aya::{
    maps::MapError,
    util::{nr_cpus, online_cpus},
};
use aya_metrics_common::Meter;
#[cfg(feature = "mocks")]
use aya_metrics_mocks::{Ebpf, PerCpuArray};
use futures::{lock::Mutex, stream::FuturesUnordered, StreamExt};
use metrics::{Counter, Label, Unit};
use thiserror::Error;
use tokio::time::{self, Duration};

#[cfg(not(feature = "mocks"))]
type PerCpuArray<V> = aya::maps::PerCpuArray<aya::maps::MapData, V>;

type AdditionalLabels = Vec<Label>;

const METRIC_LABEL_CPU: &str = "cpu";

/// Defines the dimension of a particular [`Metric`].
#[derive(Clone, Debug)]
pub enum Dimension {
    /// Dimension with additional labels.
    By(AdditionalLabels),
    /// Dimension with cpu and additional labels.
    ByCpu(AdditionalLabels),
}

type Dimensions = Vec<Dimension>;

/// Defines a metric that [`EbpfMetrics`] can report on.
#[derive(Debug)]
pub struct Metric<M: Meter> {
    /// The meter to take values from.
    meter: M,
    /// The unit with which to emit the metric.
    unit: Unit,
    /// The dimensions with which to emit the metric.
    dimensions: Dimensions,
}

impl<M: Meter> Metric<M> {
    /// Create a new [`Metric`]
    pub fn new(meter: M, unit: Unit, dimensions: Dimensions) -> Self {
        Metric {
            meter,
            unit,
            dimensions,
        }
    }
}

/// Emits custom metrics generated by an eBPF program using the [metrics] crate.
pub struct EbpfMetrics<M: Meter> {
    counters: PerCpuArray<u64>,
    metrics: Vec<Metric<M>>,
    period: Duration,
}

impl<M: Meter> EbpfMetrics<M> {
    /// Create [`EbpfMetrics<M>`] from [`Ebpf`] for specific metrics.
    ///
    /// When `EbpfMetrics<M>::run()` is invoked metrics will be periodically emitted with the given recorder.
    pub fn new(bpf: &mut Ebpf, metrics: Vec<Metric<M>>, period: Duration) -> Result<EbpfMetrics<M>, Error> {
        // Take ownership of the BPF counters map
        let counters = bpf
            .take_map(M::kind().map_name())
            .ok_or(aya::maps::MapError::InvalidName {
                name: M::kind().map_name().to_string(),
            })
            .and_then(PerCpuArray::try_from)
            .map_err(Error::MapError)?;

        Ok(EbpfMetrics {
            counters,
            metrics,
            period,
        })
    }

    /// Periodically emit metrics
    pub async fn run(self) -> Result<(), Error> {
        // Share the counters amongst the futures.
        let counters = Arc::new(Mutex::new(self.counters));

        // Create a future for each metric.
        let mut futures = FuturesUnordered::new();
        for metric in self.metrics {
            futures.push(EbpfMetrics::emit_metrics(counters.clone(), metric, self.period))
        }

        // Gracefully terminate if any future unexpectedly terminates and propagate any errors.
        // Remaining futures will be cancelled when dropped.
        futures.select_next_some().await
    }

    async fn emit_metrics(
        bpf_counters: Arc<Mutex<PerCpuArray<u64>>>,
        metric: Metric<M>,
        period: Duration,
    ) -> Result<(), Error> {
        metrics::describe_counter!(metric.meter.name(), metric.unit.clone(), metric.meter.description());

        let mut interval = time::interval(period);
        let cpu_count = nr_cpus().map_err(|(_, err)| Error::InvalidPossibleCpu(err))?;
        let cpus = online_cpus().map_err(|(_, err)| Error::InvalidOnlineCpu(err))?;

        // Pre-register all counters and store their handles for better performance
        let mut counter_handles = Vec::new();
        let mut counter_handles_by_cpu = Vec::new();
        for dimension in &metric.dimensions {
            match dimension {
                Dimension::By(labels) => {
                    let handle = metrics::counter!(metric.meter.name(), labels.clone());
                    counter_handles.push(handle);
                }
                Dimension::ByCpu(labels) => {
                    let mut handles = vec![Counter::noop(); cpu_count];
                    for cpu_id in &cpus {
                        let cpu_label = Label::new(METRIC_LABEL_CPU, cpu_id.to_string());
                        let mut labels = labels.clone();
                        labels.push(cpu_label);
                        let handle = metrics::counter!(metric.meter.name(), labels.clone());
                        handles[*cpu_id as usize] = handle;
                    }
                    counter_handles_by_cpu.push(handles);
                }
            }
        }

        // Store the previous state of the counters to calculate the delta for the next period
        let mut prev_values = vec![0u64; cpu_count];

        loop {
            interval.tick().await;

            // Get counter values per CPU
            let counter_values = {
                let guard = bpf_counters.lock().await;
                guard.get(&metric.meter.index(), 0).map_err(Error::MapError)?
            };

            // Keep a sum across CPUs
            let mut delta_sum = 0;

            // Iterate over each CPU
            for cpu_id in &cpus {
                let cpu_id = *cpu_id as usize;
                // Get the latest value for this CPU
                if let Some(value) = counter_values.get::<usize>(cpu_id) {
                    let value = *value;
                    let prev_value = prev_values[cpu_id];
                    let delta = value - prev_value;

                    // Update the sum across CPUs
                    delta_sum += delta;
                    // Store the state for the next period
                    prev_values[cpu_id] = value;

                    // Emit metric by cpu number with any additional labels
                    for handles in &mut counter_handles_by_cpu {
                        handles[cpu_id].increment(delta);
                    }
                } // GRCOV_IGNORE_LINE (apparently there is a hidden else block!)
            }

            // Emit metric with any additional labels
            for handle in &counter_handles {
                handle.increment(delta_sum);
            }
        }
    }
}

/// Errors occuring from working with EbpfMetrics
#[derive(Error, Debug)]
pub enum Error {
    /// Errors occuring while reading maps
    #[error("error opening metric array")]
    MapError(#[from] MapError),

    /// Errors occuring while listing possible CPUs
    #[error("invalid /sys/devices/system/cpu/possible format")]
    InvalidPossibleCpu(#[source] io::Error),

    /// Errors occuring while listing online CPUs
    #[error("invalid /sys/devices/system/cpu/online format")]
    InvalidOnlineCpu(#[source] io::Error),
}

// Only compile mocks when testing!
#[cfg(test)]
mod mocks;

// GRCOV_STOP_COVERAGE
#[cfg(test)]
mod test {
    use super::*;
    use aya::maps::PerCpuValues;
    use metrics::Unit;
    use metrics::{Key, Label};

    use mocks::metrics::MockRecorder;

    const HOSTNAME: &str = "this.hostname.test";
    const INTERFACE: &str = "tst0";
    const METRIC_LABEL_HOSTNAME: &str = "hostname";
    const METRIC_LABEL_INTERFACE: &str = "interface";

    #[derive(Copy, Clone, Debug)]
    enum MockCounter {
        Packets,
    }

    impl aya_metrics_common::Counter for MockCounter {
        fn name(self) -> String {
            match self {
                MockCounter::Packets => "packets".to_string(),
            }
        }

        fn index(&self) -> u32 {
            match self {
                MockCounter::Packets => 0,
            }
        }
    }

    fn get_packets_metric() -> Metric<MockCounter> {
        Metric::new(
            MockCounter::Packets,
            Unit::Count,
            vec![
                Dimension::By(vec![]),
                Dimension::By(vec![Label::new(METRIC_LABEL_HOSTNAME, HOSTNAME.to_string())]),
                Dimension::ByCpu(vec![
                    Label::new(METRIC_LABEL_HOSTNAME, HOSTNAME.to_string()),
                    Label::new(METRIC_LABEL_INTERFACE, INTERFACE.to_string()),
                ]),
            ],
        )
    }

    #[tokio::test(start_paused = true)]
    async fn test_run_registers_counters() -> Result<(), anyhow::Error> {
        let recorder = MockRecorder::new();
        let _guard = metrics::set_default_local_recorder(&recorder);

        let metrics = EbpfMetrics::new(&mut Ebpf {}, vec![get_packets_metric()], Duration::from_secs(60))?;
        tokio::spawn(async move { metrics.run().await });

        // Give the task a chance to run
        tokio::task::yield_now().await;

        // All counters get incremented once, immediately
        expect_counters(&recorder, 0)?;

        Ok(())
    }

    #[tokio::test(start_paused = true)]
    async fn test_run_failure_when_empty_map() {
        let empty_per_cpu_array = PerCpuArray::new(0, 0u64);
        let metrics = EbpfMetrics {
            counters: empty_per_cpu_array,
            metrics: vec![get_packets_metric()],
            period: Duration::from_secs(60),
        };
        let handle = tokio::spawn(async move { metrics.run().await });

        // Give the task a chance to run
        tokio::task::yield_now().await;
        handle
            .await
            .expect("Task should complete")
            .expect_err("Expected error opening metric array");
    }

    #[tokio::test(start_paused = true)]
    async fn test_emit_metrics_registers_counters() -> Result<(), anyhow::Error> {
        let recorder = MockRecorder::new();
        let _guard = metrics::set_default_local_recorder(&recorder);

        tokio::spawn(EbpfMetrics::emit_metrics(
            Arc::new(Mutex::new(PerCpuArray::new(1, 0u64))),
            get_packets_metric(),
            Duration::from_secs(60),
        ));

        // Give the task a chance to run
        tokio::task::yield_now().await;

        // All counters get incremented once, immediately
        expect_counters(&recorder, 0)?;

        Ok(())
    }

    #[tokio::test(start_paused = true)]
    async fn test_emit_metrics_increments_counters() -> Result<(), anyhow::Error> {
        let recorder = MockRecorder::new();
        let _guard = metrics::set_default_local_recorder(&recorder);

        let mut per_cpu_array = PerCpuArray::new(1, 0u64);

        tokio::spawn(EbpfMetrics::emit_metrics(
            Arc::new(Mutex::new(per_cpu_array.clone())),
            get_packets_metric(),
            Duration::from_secs(60),
        ));

        // Give the task a chance to run
        tokio::task::yield_now().await;
        // Validate the initial registration and increment (time=0s)
        expect_counters(&recorder, 0)?;

        // Update the counters
        per_cpu_array.set(0, PerCpuValues::try_from(vec![42u64; nr_cpus().map_err(|(_, err)| err)?])?, 0)?;
        // Time travel 60 seconds forward!
        time::advance(Duration::from_secs(60)).await;
        // Give the task a chance to run
        tokio::task::yield_now().await;
        // Validate the second increment (time=60s)
        expect_counters(&recorder, 42)?;

        // Update the counters
        per_cpu_array.set(0, PerCpuValues::try_from(vec![50u64; nr_cpus().map_err(|(_, err)| err)?])?, 0)?;
        // Time travel 60 seconds forward!
        time::advance(Duration::from_secs(60)).await;
        // Give the task a chance to run
        tokio::task::yield_now().await;
        // Validate the third increment (time=120s)
        expect_counters(&recorder, 42 + 8)?;

        Ok(())
    }

    fn expect_counters(recorder: &MockRecorder, packets: u64) -> Result<(), anyhow::Error> {
        let actual = recorder
            .get_counter(&Key::from_parts(
                MockCounter::Packets.name(),
                vec![Label::new(METRIC_LABEL_HOSTNAME, HOSTNAME)],
            ))
            .expect("Packet counter should be registered with hostname label");
        assert_eq!(actual, packets * online_cpus().map_err(|(_, err)| err)?.len() as u64);

        let actual = recorder
            .get_counter(&Key::from_parts(MockCounter::Packets.name(), vec![]))
            .expect("Packet counter should be registered with no labels");
        assert_eq!(actual, packets * online_cpus().map_err(|(_, err)| err)?.len() as u64);

        online_cpus().map_err(|(_, err)| err)?.iter().for_each(|cpu_id| {
            let actual = recorder
                .get_counter(&Key::from_parts(
                    MockCounter::Packets.name(),
                    vec![
                        Label::new(METRIC_LABEL_HOSTNAME, HOSTNAME),
                        Label::new(METRIC_LABEL_INTERFACE, INTERFACE),
                        Label::new(METRIC_LABEL_CPU, cpu_id.to_string()),
                    ],
                ))
                .expect("Packet counter should be registered with hostname, interface and cpu labels");
            assert_eq!(actual, packets);
        });

        Ok(())
    }
}
