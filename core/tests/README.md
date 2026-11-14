# Core's tests and the C# tests they came from

Core is ported from `src/Sobreiro.Core` one area at a time, and each area's tests come with it
from `tests/Sobreiro.Tests`. This page keeps the count, and gives a line for every C# test that
has no Rust test.

Core's tests build into one program, `core/tests/all`. The bridge's exe has its own, in
`bridge/tests`.

```
cargo test -p sobreiro-core
cargo test -p sobreiro-bridge
```

A C# `[Theory]` with rows is one Rust test that walks the same rows, so the Rust count is lower
than the count the C# test run reports. "Cases" below is that C# count: one for each `[Fact]` and
one for each row.

## Limits, usage and the files the bridge writes

The whole area: the limits, `usage.json` (its format, reading and writing it, merging a reading
into it), the atomic write, and the bridge.

| C# test file | Rust file | C# cases | Ported | Rust tests |
| --- | --- | ---: | ---: | ---: |
| `LimitListTests.cs` | `core/tests/all/limit_list.rs` | 28 | 28 | 14 |
| `UsageLevelTests.cs` | `core/tests/all/usage_level.rs` | 8 | 8 | 3 |
| `RateLimitsJsonTests.cs` | `core/tests/all/rate_limits_json.rs` | 20 | 20 | 14, and 4 new |
| `ResetPeriodTests.cs` | `core/tests/all/reset_period.rs` | 4 | 4 | 4 |
| `UsageFileTests.cs` | `core/tests/all/usage_file.rs`, `app_paths.rs` | 13 | 13 | 13, and 1 new |
| `UsageMergeTests.cs` | `core/tests/all/usage_merge.rs` | 16 | 16 | 16 |
| `AtomicFileTests.cs` | `core/tests/all/atomic_file.rs` | 6 | 6 | 6 |
| `BridgeRunnerTests.cs` | `core/tests/all/bridge_runner.rs` | 10 | 9 | 9 |
| `BridgeConfigTests.cs` | `core/tests/all/bridge_config.rs` | 6 | 6 | 4 |
| `BashLocatorTests.cs` | `core/tests/all/bash_locator.rs` | 6 | 6 | 6 |
| `ChainRunnerTests.cs` | `bridge/tests/chain_runner.rs` | 10 | 10 | 9, and 1 new |
| | | 127 | 126 | 98, and 6 new |

Ported ahead of its own area, because the bridge prints it:

| C# test file | Rust file | C# cases | Ported | Rust tests |
| --- | --- | ---: | ---: | ---: |
| `StatusLineTextTests.cs` (Display) | `core/tests/all/status_line_text.rs` | 4 | 4 | 4 |

The three `AppPaths_*` tests in `UsageFileTests.cs` are in `app_paths.rs`. `AppPathsTests.cs`
itself waits for the area "Paths and config", with the rest of `AppPaths`.

### C# tests with no Rust test

- `BridgeRunnerTests.Still_prints_when_chain_throws`: a Rust function has no exception to
  throw. A chained command that fails gives "nothing", and
  `bridge_runner::falls_back_to_default_line_when_chain_cannot_start` covers that.

### C# tests ported in part

- `ChainRunnerTests.StartInfo_uses_bash_dash_c` also asserts that the process's input and
  output are redirected and that it is not started through the shell. A command in Rust does
  not let a test read those settings back. The tests that run a command show them:
  `run_passes_stdin_and_returns_stdout_and_exit_code` would fail without either pipe.
- The three `AppPaths_*` tests set `SOBREIRO_DATA_DIR` for the whole test run and ask Windows
  for the user's folder. In Rust both are handed to the code under test, so the tests name
  them outright and no test changes what another reads.

### Rust tests with no C# test

They pin down what the C# does where no C# test names it, so the port cannot drift there.

- `rate_limits_json::write_usage_file_gives_the_bytes_the_csharp_bridge_writes`: a `usage.json`
  written by the 1.1.0 bridge is read and written again, and comes out the same to the byte.
- `rate_limits_json::write_usage_file_refuses_a_number_json_cannot_hold`: the C# writer throws
  on infinity and on "not a number"; here the writer gives an error and nothing is written.
- `rate_limits_json::a_time_with_a_fraction_is_cut_to_the_second`.
- `rate_limits_json::a_time_outside_the_years_1_to_9999_is_not_a_time`: the C# holds no date
  outside those years and reads such a time as missing.
- `usage_file::read_skips_a_byte_order_mark`: the C# reader skips the three bytes some editors
  put at the start of a UTF-8 file.
- `chain_runner::a_command_out_of_time_takes_the_processes_it_started_with_it`: the C# ends
  the chained command with every process it started; this shows the Rust does.

And two files of tests the C# had no like of:

- `bridge/tests/exe.rs`, 4 tests: the exe itself, run as Claude Code runs it, with a data
  folder of its own. One of them holds the file the 1.1.0 bridge wrote for the same input.
- `bridge/src/windows_calls.rs`, 1 test: Windows says where the user's local application data
  folder is.
