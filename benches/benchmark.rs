
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fatigue::interpolate::{InterpolationStrategy, Linear, NDInterpolation};
use fatigue::timeseries::Point;
use fatigue::rainflow::rainflow;
use rand::distributions::{Distribution, Uniform}; // 0.6.5

fn setup_large_dataset(interpolator: &mut NDInterpolation) {
    // Example setup function - replace with your actual dataset setup
    for x in 1..=100000 {
        let point = Point { coordinates: vec![x as f64], file: None };
        interpolator.add_point(point, 2.0 * x as f64);
    }
}

fn bench_linear_interpolation(c: &mut Criterion) {
    c.bench_function("linear interpolation large dataset", |b| {
        let strategy: Box<dyn InterpolationStrategy> = Box::new(Linear);
        let mut interpolator = NDInterpolation::new(&strategy);
        setup_large_dataset(&mut interpolator);

        let target_point = Point { coordinates: vec![500.0], file: None }; // Example target point

        b.iter(|| {
            interpolator.interpolate(&black_box(target_point.clone())).unwrap();
        });
    });
}

fn bench_rainflow(c: &mut Criterion) {
    c.bench_function("Rainflow counting algorithm on large dataset", |b| {

        let step = Uniform::new(0.0, 50.0);
        let mut rng = rand::thread_rng();
        let choices: Vec<f64> = step.sample_iter(&mut rng).take(100000).collect();
        b.iter(|| {
            let (_means, _ranges) = rainflow(&choices);
        });
    });
}

criterion_group!(benches, bench_linear_interpolation, bench_rainflow);
criterion_main!(benches);
