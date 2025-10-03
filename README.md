# `kairoi`

Kairoi is an application-level tracing library.

## How to use?

Just use as `println!`:

```rust
info!("Hello, World!");
warn!("It's warning: {}", "blabla");
```

Or in a span (equivalent of `Span` in another logging libraries such as `tracing`):

```rust
Span::scope(async |scope: Scope| {
    let data = Span::default().with_name("Hello World".to_string());

    scope.update(data.with_progress(Progress::new(100, 0)));
    for i in 1..=100 {
        if i % 10 == 0 {
            info!("{}/100", i);
        }

        sleep(Duration::from_millis(50)).await;
        scope.update(data.with_progress(Progress::new(100, i)));
    }
})
.await;
```

More examples are at `/examples`
