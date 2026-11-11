use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rnfc_fdir::{SensorHealth, Failsafe, Health, SensorVoter, SpoofingDetector};

fn bench_sensor_health(c: &mut Criterion) {
    let mut h = SensorHealth::new(50.0);
    c.bench_function("sensor_health_update", |b| {
        b.iter(|| h.update(black_box(50.5)))
    });
}

fn bench_failsafe(c: &mut Criterion) {
    let fs = Failsafe::new();
    c.bench_function("failsafe_evaluate", |b| {
        b.iter(|| fs.evaluate(black_box([100.0, 50.0]), black_box(12.0), black_box(Health::Healthy)))
    });
}

fn bench_sensor_voter(c: &mut Criterion) {
    let v = SensorVoter::new();
    c.bench_function("sensor_voter_vote", |b| {
        b.iter(|| v.vote(black_box(50.0), black_box(50.5), black_box(49.8)))
    });
}

fn bench_spoofing_detector(c: &mut Criterion) {
    let mut sd = SpoofingDetector::new();
    c.bench_function("spoofing_detector_update", |b| {
        b.iter(|| sd.update(black_box(0.5), black_box(0.0)))
    });
}

criterion_group!(benches, bench_sensor_health, bench_failsafe, bench_sensor_voter, bench_spoofing_detector);
criterion_main!(benches);
