use std::{
    hint::black_box,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use criterion::{Criterion, criterion_group, criterion_main};
use gpui_core::{BackgroundExecutor, ForegroundExecutor, ThreadedDispatcher};

fn benchmark_async_tasks(criterion: &mut Criterion) {
    let dispatcher = Arc::new(ThreadedDispatcher::new());
    let background = BackgroundExecutor::new(dispatcher.clone());
    let foreground = ForegroundExecutor::new(dispatcher.clone());

    criterion.bench_function("background_task_completion", |bencher| {
        bencher.iter(|| {
            let done = Arc::new(AtomicBool::new(false));
            let done_for_task = done.clone();
            background
                .spawn(async move {
                    done_for_task.store(true, Ordering::Release);
                })
                .detach();
            dispatcher.run_until(|| done.load(Ordering::Acquire));
            black_box(done.load(Ordering::Relaxed));
        });
    });

    criterion.bench_function("foreground_task_completion", |bencher| {
        bencher.iter(|| {
            let done = Arc::new(AtomicBool::new(false));
            let done_for_task = done.clone();
            foreground
                .spawn(async move {
                    done_for_task.store(true, Ordering::Release);
                })
                .detach();
            dispatcher.run_until(|| done.load(Ordering::Acquire));
            black_box(done.load(Ordering::Relaxed));
        });
    });

    criterion.bench_function("background_batch_64_completion", |bencher| {
        bencher.iter(|| {
            const TASKS: usize = 64;
            let completed = Arc::new(AtomicUsize::new(0));
            for _ in 0..TASKS {
                let completed = completed.clone();
                background
                    .spawn(async move {
                        completed.fetch_add(1, Ordering::Release);
                    })
                    .detach();
            }
            dispatcher.run_until(|| completed.load(Ordering::Acquire) == TASKS);
            black_box(completed.load(Ordering::Relaxed));
        });
    });
}

criterion_group!(async_tasks, benchmark_async_tasks);
criterion_main!(async_tasks);
