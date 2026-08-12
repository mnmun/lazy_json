use a_bc::lexer::Builder;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use json::JsonLexer;

fn generate_array_with_small_objects(n: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(n * 35);
    data.push(b'[');
    for i in 0..n {
        if i > 0 {
            data.push(b',');
        }
        data.extend_from_slice(b"{\"k\":");
        data.extend_from_slice(i.to_string().as_bytes());
        data.push(b'}');
    }
    data.push(b']');
    data
}

fn generate_nested_object(depth: usize, breadth: usize) -> Vec<u8> {
    fn build(depth: usize, breadth: usize, data: &mut Vec<u8>) {
        if depth == 0 {
            data.extend_from_slice(b"\"leaf\"");
            return;
        }
        data.push(b'{');
        for i in 0..breadth {
            if i > 0 {
                data.push(b',');
            }
            data.extend_from_slice(b"\"k");
            data.extend_from_slice(i.to_string().as_bytes());
            data.extend_from_slice(b"\":");
            build(depth - 1, breadth, data);
        }
        data.push(b'}');
    }

    let mut data = Vec::new();
    build(depth, breadth, &mut data);
    data
}

fn generate_array_with_big_objects(n: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(n * 230);
    data.push(b'[');
    for i in 0..n {
        if i > 0 {
            data.push(b',');
        }
        data.push(b'{');
        data.extend_from_slice(b"\"text\":\"");
        for j in 0u8..100 {
            if j % 10 == 0 {
                data.extend_from_slice(b"\\\"");
            } else {
                data.push(b'a' + j % 26);
            }
        }
        data.extend_from_slice(b"\",\"id\":");
        data.extend_from_slice(i.to_string().as_bytes());
        data.push(b'}');
    }
    data.push(b']');
    data
}

fn bench_parse(c: &mut Criterion) {
    let array_100 = generate_array_with_small_objects(100);
    let array_500 = generate_array_with_small_objects(500);
    let array_1000 = generate_array_with_small_objects(1000);
    let array_5000 = generate_array_with_small_objects(5000);
    let array_10000 = generate_array_with_small_objects(10000);
    let array_50000 = generate_array_with_small_objects(50000);
    let array_100000 = generate_array_with_small_objects(100000);
    let array_500000 = generate_array_with_small_objects(500000);

    let nested_object_3x3 = generate_nested_object(3, 3);
    let nested_object_5x3 = generate_nested_object(5, 3);
    let nested_object_6x4 = generate_nested_object(6, 4);

    let heavy_array_100 = generate_array_with_big_objects(100);
    let heavy_array_1000 = generate_array_with_big_objects(1000);

    let arrays: &[(&str, &[u8])] = &[
        ("100", &array_100),
        ("500", &array_500),
        ("1000", &array_1000),
        ("5000", &array_5000),
        ("10000", &array_10000),
        ("50000", &array_50000),
        ("100000", &array_100000),
        ("500000", &array_500000),
    ];

    {
        let mut group = c.benchmark_group("Array");
        group.sample_size(20);

        for &(size, data) in arrays {
            group.bench_with_input(
                BenchmarkId::new("Size", size),
                &data,
                |b, &data| {
                    b.iter(|| {
                        let lexer =
                            JsonLexer::new(Builder::new(data).build().unwrap());
                        criterion::black_box(lexer.count())
                    });
                },
            );
        }
    }

    let nested_objects: &[(&str, &[u8])] = &[
        ("3x3", &nested_object_3x3),
        ("5x3", &nested_object_5x3),
        ("6x4", &nested_object_6x4),
    ];

    {
        let mut group = c.benchmark_group("Nested object");
        group.sample_size(20);

        for &(size, data) in nested_objects {
            group.bench_with_input(
                BenchmarkId::new("Size", size),
                &data,
                |b, &data| {
                    b.iter(|| {
                        let lexer =
                            JsonLexer::new(Builder::new(data).build().unwrap());
                        criterion::black_box(lexer.count())
                    });
                },
            );
        }
    }

    let heavy_arrays: &[(&str, &[u8])] =
        &[("100", &heavy_array_100), ("1000", &heavy_array_1000)];

    {
        let mut group = c.benchmark_group("Heavy array");
        group.sample_size(20);

        for &(size, data) in heavy_arrays {
            group.bench_with_input(
                BenchmarkId::new("Size", size),
                &data,
                |b, &data| {
                    b.iter(|| {
                        let lexer =
                            JsonLexer::new(Builder::new(data).build().unwrap());
                        criterion::black_box(lexer.count())
                    });
                },
            );
        }
    }
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
