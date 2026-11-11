use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rnfc_abstractions::{GnssFix, ImuSample};
use rnfc_core::attitude_controller::AttitudeController;
use rnfc_core::fusion_mag::FusionMag;
use rnfc_core::guidance::Guidance;
use rnfc_core::NavCore;

fn bench_navcore_step(c: &mut Criterion) {
    let mut nav = NavCore::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    let imu = ImuSample { accel: [0.0, 0.0, -9.81], gyro: [0.1, -0.05, 0.02], dt: 0.01 };
    let gnss = Some(GnssFix { position: [100.0, 50.0, 0.0], velocity: [2.0, 1.0, 0.0], hdop: 1.0 });
    c.bench_function("navcore_step", |b| {
        b.iter(|| nav.step(black_box(&imu), black_box(gnss.as_ref())))
    });
}

fn bench_fusion_mag_update(c: &mut Criterion) {
    let mut f = FusionMag::new(0.0);
    c.bench_function("fusion_mag_update", |b| {
        b.iter(|| f.update(black_box(50.0), black_box(30.0), black_box(0.1), black_box(0.01)))
    });
}

fn bench_guidance_update(c: &mut Criterion) {
    let mut g = Guidance::new();
    c.bench_function("guidance_update", |b| {
        b.iter(|| g.update(black_box([50.0, 25.0, 0.0])))
    });
}

fn bench_attitude_controller(c: &mut Criterion) {
    let mut ctrl = AttitudeController::new();
    let sp = [0.05, 0.1, 0.02];
    let m = [0.03, 0.08, 0.01];
    c.bench_function("attitude_controller_update", |b| {
        b.iter(|| ctrl.update(black_box(sp), black_box(m), black_box(0.01)))
    });
}

criterion_group!(benches, bench_navcore_step, bench_fusion_mag_update, bench_guidance_update, bench_attitude_controller);
criterion_main!(benches);
