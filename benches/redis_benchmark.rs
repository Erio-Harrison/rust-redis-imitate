use criterion::{criterion_group, criterion_main, Criterion};
use redis_clone::storage::memory::MemoryStorage;
use redis_clone::commands::parser::Command;
use redis_clone::commands::executor::CommandExecutor;
use std::sync::{Arc, Mutex};

fn bench_set(c: &mut Criterion) {
    let storage = Arc::new(Mutex::new(MemoryStorage::new()));
    let executor = CommandExecutor::new(Arc::clone(&storage));

    c.bench_function("SET", |b| {
        b.iter(|| {
            executor.execute_command(Command::Set("test_key".to_string(), "test_value".to_string()))
        })
    });
}

fn bench_get(c: &mut Criterion) {
    let storage = Arc::new(Mutex::new(MemoryStorage::new()));
    let executor = CommandExecutor::new(Arc::clone(&storage));

    executor.execute_command(Command::Set("test_key".to_string(), "test_value".to_string()));

    c.bench_function("GET", |b| {
        b.iter(|| {
            executor.execute_command(Command::Get("test_key".to_string()))
        })
    });
}

fn bench_lpush(c: &mut Criterion) {
    let storage = Arc::new(Mutex::new(MemoryStorage::new()));
    let executor = CommandExecutor::new(Arc::clone(&storage));

    c.bench_function("LPUSH", |b| {
        b.iter(|| {
            executor.execute_command(Command::LPush("test_list".to_string(), "test_value".to_string()))
        })
    });
}

fn bench_rpop(c: &mut Criterion) {
    let storage = Arc::new(Mutex::new(MemoryStorage::new()));
    let executor = CommandExecutor::new(Arc::clone(&storage));

    executor.execute_command(Command::LPush("test_list".to_string(), "test_value".to_string()));

    c.bench_function("RPOP", |b| {
        b.iter(|| {
            executor.execute_command(Command::RPop("test_list".to_string()))
        })
    });
}

criterion_group!(benches, bench_set, bench_get, bench_lpush, bench_rpop);
criterion_main!(benches);