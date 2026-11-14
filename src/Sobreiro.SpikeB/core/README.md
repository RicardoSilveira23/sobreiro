# sobreiro-core (spike)

The part of Sobreiro.Core a shell needs, in Rust. Polling keeps the C# split: blocking work off the UI thread,
state changes on it. The poller is not `Send`; `PollRun` and `PollDone` are.

```rust
let mut poller = ProviderPoller::new(provider,            // ClaudeProvider or CopilotProvider
    Box::new(|run: PollRun| { /* move it to a worker, then post the result to the UI thread: */
        std::thread::spawn(move || { let done = run.run(); post_to_ui(move || POLLER.done(done, now())); });
    }),
    Box::new(|| redraw()), None);                          // `changed`, optional trace
poller.start(now());  // on the UI thread; then poller.tick(now()) every ~30 s, poller.flyout_opened(now()) on open
let limits = poller.limits(now());  poller.state(); poller.misses();   // read on the UI thread
```
Copilot also: `let job = copilot.read_cache_job();` run `job()` on a worker, then `take_early(reading)` on the UI thread. `poller.stop()` / `poller.dispose()` cancel a running poll; its answer is dropped by `done`.
