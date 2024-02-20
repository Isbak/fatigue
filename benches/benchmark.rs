
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fatigue::interpolate::{InterpolationStrategyEnum, Linear, NDInterpolation};
use fatigue::timeseries::Point;
use fatigue::rainflow::rainflow;
use rand::distributions::{Distribution, Uniform}; // 0.6.5

fn setup_large_dataset(interpolator: &mut NDInterpolation) {
    for x in 1..=100 {
        // Simulating a 3D dataset with a straightforward relationship
        let coordinates = vec![x as f64, x as f64 * 2.0, x as f64 * 3.0]; // Example linear relationship in 3D
        let point = Point { coordinates, file: None }; // Assuming Point::coordinates is Vec<f64>
        let value = x as f64 * 2.0; // Value is a function of x for simplicity
        
        interpolator.add_point(point, value);
    }
}

fn setup_large_dataset_target() -> Vec<Vec<f64>> {
    let mut targets = Vec::new();
    let start = 10.0;
    let end = 90.0;
    let step = 0.1;

    let mut x = start;
    while x <= end {
        targets.push(vec![x, x * 2.0, x * 3.0]);
        x += step;
    }

    targets
}


fn bench_linear_interpolation(c: &mut Criterion) {
    c.bench_function("linear interpolation large dataset", |b| {
        let strategy = InterpolationStrategyEnum::Linear(Linear);
        let mut interpolator = NDInterpolation::new(&strategy);
        setup_large_dataset(&mut interpolator);
        let target_point = setup_large_dataset_target();

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
