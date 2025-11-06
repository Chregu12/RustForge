/// Memory Profiling Tests using dhat-rs
///
/// These tests measure heap allocation patterns and memory usage
/// to identify optimization opportunities.
///
/// Run with: cargo test --test memory_profile --features dhat-heap

#[cfg(feature = "dhat-heap")]
use dhat::{Alloc, Profiler};

#[global_allocator]
#[cfg(feature = "dhat-heap")]
static ALLOC: Alloc = Alloc;

#[test]
#[cfg(feature = "dhat-heap")]
fn profile_command_dispatch() {
    let _profiler = Profiler::new_heap();

    // Simulate command dispatch workload
    for i in 0..1000 {
        let command = format!("test:command:{}", i);
        let args = vec![
            format!("arg1:{}", i),
            format!("arg2:{}", i),
            format!("arg3:{}", i),
        ];

        // Parse input
        use foundry_api::input::InputParser;
        let _parser = InputParser::from_args(&args);

        // Simulate some work
        std::hint::black_box(&command);
        std::hint::black_box(&args);
    }

    // Stats printed on drop
}

#[test]
#[cfg(feature = "dhat-heap")]
fn profile_optimized_input_parsing() {
    let _profiler = Profiler::new_heap();

    // Test optimized input parser memory usage
    for i in 0..1000 {
        let args = vec![
            format!("command:{}", i),
            "--force".to_string(),
            "--verbose".to_string(),
        ];

        use foundry_api::optimized_input::OptimizedInputParser;
        let parser = OptimizedInputParser::from_args(&args);

        // Verify stack allocation
        assert!(parser.is_stack_allocated(), "Should be stack-allocated for small inputs");

        std::hint::black_box(&parser);
    }
}

#[test]
#[cfg(feature = "dhat-heap")]
fn profile_service_container() {
    let _profiler = Profiler::new_heap();

    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        use foundry_service_container::FastContainer;

        let container = FastContainer::new();

        #[derive(Clone)]
        struct TestService {
            value: String,
        }

        // Bind multiple services
        for i in 0..100 {
            let key = format!("service:{}", i);
            container
                .singleton(key, || {
                    Ok(TestService {
                        value: format!("value:{}", i),
                    })
                })
                .await
                .unwrap();
        }

        // Resolve services multiple times
        for i in 0..100 {
            let key = format!("service:{}", i);
            let _service: std::sync::Arc<TestService> =
                container.resolve(&key).await.unwrap();
        }

        std::hint::black_box(&container);
    });
}

#[test]
#[cfg(feature = "dhat-heap")]
fn profile_cow_identifiers() {
    let _profiler = Profiler::new_heap();

    use foundry_domain::cow_identifiers::CommandId;

    // Borrowed identifiers (zero allocation)
    for i in 0..1000 {
        let id = CommandId::borrowed("migrate:run");
        std::hint::black_box(&id);
    }

    // Owned identifiers (one allocation each)
    for i in 0..1000 {
        let id = CommandId::owned(format!("command:{}", i));
        std::hint::black_box(&id);
    }
}

#[test]
#[cfg(not(feature = "dhat-heap"))]
fn memory_profiling_disabled() {
    println!("Memory profiling requires 'dhat-heap' feature");
    println!("Run with: cargo test --test memory_profile --features dhat-heap");
}
