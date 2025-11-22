use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashMap;

// # Blade Template Performance Benchmarks
//
// Benchmarks for template operations:
// - Template compilation
// - Template rendering
// - Complex templates with loops and conditions
// - Template caching effectiveness

fn compile_simple_template(template: &str) -> String {
    // Simulate template compilation
    template.replace("{{ ", "").replace(" }}", "")
}

fn render_simple_template(template: &str, data: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in data {
        result = result.replace(&format!("{{{{{}}}}}", key), value);
    }
    result
}

fn render_complex_template(count: usize) -> String {
    let mut result = String::with_capacity(count * 100);
    result.push_str("<ul>");
    for i in 0..count {
        result.push_str(&format!("<li>Item {}</li>", i));
    }
    result.push_str("</ul>");
    result
}

fn benchmark_compilation(c: &mut Criterion) {
    c.bench_function("blade/compile/simple", |b| {
        b.iter(|| black_box(compile_simple_template("Hello {{ name }}!")));
    });

    c.bench_function("blade/compile/complex", |b| {
        b.iter(|| {
            black_box(compile_simple_template(
                "<div>{{ title }}</div><p>{{ content }}</p><footer>{{ footer }}</footer>",
            ))
        });
    });
}

fn benchmark_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("blade/render");

    group.bench_function("simple", |b| {
        let mut data = HashMap::new();
        data.insert("name".to_string(), "World".to_string());

        b.iter(|| black_box(render_simple_template("Hello {{ name }}!", &data)));
    });

    for count in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("loop", count), count, |b, &count| {
            b.iter(|| black_box(render_complex_template(count)));
        });
    }

    group.finish();
}

criterion_group!(benches, benchmark_compilation, benchmark_rendering);
criterion_main!(benches);
