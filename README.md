# rust_thread_pool

- 用法

```rust

        let pool = ThreadPoolBuilder::new().
            with_pool_name("haha".to_string()).
            with_core_pool_size(4).
            build();
        for i in 0..100 {
            pool.execute(move || {
                println!("execute job {}", i);
            });
        }
        // pool.update_core_pool_size(0);
        // assert_eq!(pool.data.has_worker(), true);
        pool.join();
```
