//! Benchmarks for the vz data pipeline: loading, inference, and full pipeline.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use vz::loader::{self, InputFormat};
use vz::pipeline;

/// Generate a 1000-row CSV string: date,city,revenue
fn generate_csv_1000() -> String {
    let cities = ["Tokyo", "Osaka", "Kyoto", "Nagoya", "Fukuoka"];
    let mut lines = Vec::with_capacity(1001);
    lines.push("date,city,revenue".to_string());
    for i in 0..1000 {
        let month = (i % 12) + 1;
        let year = 2020 + i / 12;
        let city = cities[i % cities.len()];
        let revenue = 1000 + (i * 37) % 9000;
        lines.push(format!("{year}-{month:02}-01,{city},{revenue}"));
    }
    lines.join("\n")
}

/// Generate a 1000-row JSON array: [{date, city, revenue}, ...]
fn generate_json_1000() -> String {
    let cities = ["Tokyo", "Osaka", "Kyoto", "Nagoya", "Fukuoka"];
    let mut objects = Vec::with_capacity(1000);
    for i in 0..1000 {
        let month = (i % 12) + 1;
        let year = 2020 + i / 12;
        let city = cities[i % cities.len()];
        let revenue = 1000 + (i * 37) % 9000;
        objects.push(format!(
            r#"{{"date":"{year}-{month:02}-01","city":"{city}","revenue":{revenue}}}"#
        ));
    }
    format!("[{}]", objects.join(","))
}

/// Generate a 1000-row space-aligned table
fn generate_space_1000() -> String {
    let cities = ["Tokyo   ", "Osaka   ", "Kyoto   ", "Nagoya  ", "Fukuoka "];
    let mut lines = Vec::with_capacity(1001);
    lines.push("DATE         CITY      REVENUE".to_string());
    for i in 0..1000 {
        let month = (i % 12) + 1;
        let year = 2020 + i / 12;
        let city = cities[i % cities.len()];
        let revenue = 1000 + (i * 37) % 9000;
        lines.push(format!("{year}-{month:02}-01   {city}  {revenue:>7}"));
    }
    lines.join("\n")
}

fn bench_csv_parse(c: &mut Criterion) {
    let csv_data = generate_csv_1000();
    c.bench_function("csv_parse_1000", |b| {
        b.iter(|| loader::load_from_content(black_box(&csv_data), InputFormat::Csv, false).unwrap())
    });
}

fn bench_json_parse(c: &mut Criterion) {
    let json_data = generate_json_1000();
    c.bench_function("json_parse_1000", |b| {
        b.iter(|| {
            loader::load_from_content(black_box(&json_data), InputFormat::Json, false).unwrap()
        })
    });
}

fn bench_space_parse(c: &mut Criterion) {
    let space_data = generate_space_1000();
    c.bench_function("space_parse_1000", |b| {
        b.iter(|| {
            loader::load_from_content(black_box(&space_data), InputFormat::Space, false).unwrap()
        })
    });
}

fn bench_infer(c: &mut Criterion) {
    let csv_data = generate_csv_1000();
    let data = loader::load_from_content(&csv_data, InputFormat::Csv, false).unwrap();
    c.bench_function("infer_1000", |b| {
        b.iter(|| pipeline::infer_from_data(black_box(&data)))
    });
}

fn bench_pipeline(c: &mut Criterion) {
    let csv_data = generate_csv_1000();
    c.bench_function("pipeline_csv_1000", |b| {
        b.iter(|| {
            let data =
                loader::load_from_content(black_box(&csv_data), InputFormat::Csv, false).unwrap();
            let _schema = pipeline::infer_from_data(&data);
        })
    });
}

fn bench_infer_large(c: &mut Criterion) {
    // Test that inference on large data is still fast (only samples 100 rows)
    let cities = ["Tokyo", "Osaka", "Kyoto", "Nagoya", "Fukuoka"];
    let mut lines = Vec::with_capacity(10001);
    lines.push("date,city,revenue".to_string());
    for i in 0..10000 {
        let month = (i % 12) + 1;
        let year = 2020 + i / 12;
        let city = cities[i % cities.len()];
        let revenue = 1000 + (i * 37) % 9000;
        lines.push(format!("{year}-{month:02}-01,{city},{revenue}"));
    }
    let csv_data = lines.join("\n");
    let data = loader::load_from_content(&csv_data, InputFormat::Csv, false).unwrap();

    c.bench_function("infer_10000_rows", |b| {
        b.iter(|| pipeline::infer_from_data(black_box(&data)))
    });
}

/// Benchmark the full load→infer pipeline to measure combined throughput
fn bench_full_load_infer(c: &mut Criterion) {
    let csv_data = generate_csv_1000();
    let json_data = generate_json_1000();

    let mut group = c.benchmark_group("full_pipeline");
    group.bench_function("csv_load_infer_1000", |b| {
        b.iter(|| {
            let data =
                loader::load_from_content(black_box(&csv_data), InputFormat::Csv, false).unwrap();
            pipeline::infer_from_data(&data)
        })
    });
    group.bench_function("json_load_infer_1000", |b| {
        b.iter(|| {
            let data =
                loader::load_from_content(black_box(&json_data), InputFormat::Json, false).unwrap();
            pipeline::infer_from_data(&data)
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_csv_parse,
    bench_json_parse,
    bench_space_parse,
    bench_infer,
    bench_infer_large,
    bench_pipeline,
    bench_full_load_infer,
);
criterion_main!(benches);
