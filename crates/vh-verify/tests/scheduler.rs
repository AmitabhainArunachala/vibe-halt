#![forbid(unsafe_code)]

use vh_core::{Scheduler, VirtualTime};

fn permutations(values: &mut [u8], start: usize, output: &mut Vec<Vec<u8>>) {
    if start == values.len() {
        output.push(values.to_vec());
        return;
    }

    for index in start..values.len() {
        values.swap(start, index);
        permutations(values, start + 1, output);
        values.swap(start, index);
    }
}

fn all_five_event_orders() -> Vec<Vec<u8>> {
    let mut values = [0, 1, 2, 3, 4];
    let mut output = Vec::new();
    permutations(&mut values, 0, &mut output);
    output
}

#[test]
fn every_equal_time_permutation_fires_in_insertion_order() {
    let mut orders = all_five_event_orders();
    assert_eq!(orders.len(), 120);
    orders.sort();
    orders.dedup();
    assert_eq!(
        orders.len(),
        120,
        "permutation generator emitted duplicates"
    );

    for order in orders {
        for at in [VirtualTime::ZERO, VirtualTime(10), VirtualTime(u64::MAX)] {
            let mut scheduler = Scheduler::new();
            for event in &order {
                scheduler.schedule(at, *event);
            }
            let fired: Vec<u8> = (0..order.len())
                .map(|_| scheduler.pop().expect("scheduled event").1)
                .collect();
            assert_eq!(fired, order);
            assert!(scheduler.is_empty());
        }
    }
}

#[test]
fn every_equal_time_permutation_survives_schedule_pop_interleaving() {
    for order in all_five_event_orders() {
        for split in 1..=order.len() {
            let mut scheduler = Scheduler::new();
            for event in &order[..split] {
                scheduler.schedule(VirtualTime(10), *event);
            }

            let mut fired = vec![scheduler.pop().expect("scheduled prefix").1];
            for event in &order[split..] {
                scheduler.schedule(VirtualTime(10), *event);
            }
            while let Some((_, event)) = scheduler.pop() {
                fired.push(event);
            }

            assert_eq!(fired, order);
        }
    }
}

fn mixed_time_nanos(event: u8) -> u64 {
    match event {
        0 | 3 => 0,
        1 => 10,
        2 => u64::MAX,
        4 => 10,
        _ => unreachable!("five-event fixture"),
    }
}

fn mixed_time(event: u8) -> VirtualTime {
    VirtualTime(mixed_time_nanos(event))
}

#[test]
fn every_mixed_time_permutation_matches_the_time_then_insertion_model() {
    for order in all_five_event_orders() {
        let mut scheduler = Scheduler::new();
        let mut expected: Vec<(u64, usize, u8)> = order
            .iter()
            .copied()
            .enumerate()
            .map(|(insertion, event)| (mixed_time_nanos(event), insertion, event))
            .collect();

        for event in &order {
            scheduler.schedule(mixed_time(*event), *event);
        }
        expected.sort_unstable_by_key(|(time, insertion, _)| (*time, *insertion));

        let actual: Vec<(u64, u8)> = std::iter::from_fn(|| scheduler.pop())
            .map(|(time, event)| (time.0, event))
            .collect();
        let expected: Vec<(u64, u8)> = expected
            .into_iter()
            .map(|(time, _, event)| (time, event))
            .collect();
        assert_eq!(actual, expected, "insertion order {order:?}");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    Schedule,
    Pop,
}

fn legal_operation_traces(
    scheduled: usize,
    popped: usize,
    current: &mut Vec<Operation>,
    output: &mut Vec<Vec<Operation>>,
) {
    const EVENTS: usize = 5;

    if popped == EVENTS {
        output.push(current.clone());
        return;
    }
    if scheduled < EVENTS {
        current.push(Operation::Schedule);
        legal_operation_traces(scheduled + 1, popped, current, output);
        current.pop();
    }
    if popped < scheduled {
        current.push(Operation::Pop);
        legal_operation_traces(scheduled, popped + 1, current, output);
        current.pop();
    }
}

fn all_legal_operation_traces() -> Vec<Vec<Operation>> {
    let mut output = Vec::new();
    legal_operation_traces(0, 0, &mut Vec::new(), &mut output);
    output
}

#[test]
fn every_legal_schedule_pop_trace_matches_an_independent_queue_model() {
    let operation_traces = all_legal_operation_traces();
    assert_eq!(operation_traces.len(), 42);
    let mut equal_time_checked = 0usize;
    let mut mixed_time_checked = 0usize;

    for order in all_five_event_orders() {
        for operations in &operation_traces {
            for mixed_times in [false, true] {
                let mut scheduler = Scheduler::new();
                let mut reference = Vec::new();
                let mut next_schedule = 0usize;
                let mut watermark = 0u64;
                let mut temporally_legal = true;

                for operation in operations {
                    match operation {
                        Operation::Schedule => {
                            let event = order[next_schedule];
                            let time = if mixed_times {
                                mixed_time_nanos(event)
                            } else {
                                10
                            };
                            if time < watermark {
                                temporally_legal = false;
                                break;
                            }
                            scheduler.schedule(VirtualTime(time), event);
                            reference.push((time, next_schedule, event));
                            next_schedule += 1;
                        }
                        Operation::Pop => {
                            let expected_index = reference
                                .iter()
                                .enumerate()
                                .min_by_key(|(_, (time, insertion, _))| (*time, *insertion))
                                .map(|(index, _)| index)
                                .expect("legal trace never pops an empty model");
                            let (time, _, event) = reference.remove(expected_index);
                            watermark = time;
                            assert_eq!(
                                scheduler.pop().map(|(time, event)| (time.0, event)),
                                Some((time, event)),
                                "order={order:?} operations={operations:?} mixed={mixed_times}"
                            );
                        }
                    }
                }

                if !temporally_legal {
                    continue;
                }
                assert!(reference.is_empty());
                assert!(scheduler.is_empty());
                if mixed_times {
                    mixed_time_checked += 1;
                } else {
                    equal_time_checked += 1;
                }
            }
        }
    }

    assert_eq!(equal_time_checked, 5_040);
    assert_eq!(mixed_time_checked, 1_440);
}

#[test]
fn production_rejects_scheduling_before_the_raw_time_watermark() {
    let mut scheduler = Scheduler::new();
    scheduler.schedule(VirtualTime(10), 0u8);
    assert_eq!(scheduler.pop().map(|(time, _)| time.0), Some(10));

    let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        scheduler.schedule(VirtualTime(9), 1u8);
    }));
    assert!(rejected.is_err());
}
