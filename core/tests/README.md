# Core's tests and the C# tests they came from

Core is ported from `src/Sobreiro.Core` one area at a time, and each area's tests come with it
from `tests/Sobreiro.Tests`. This page keeps the count, and gives a line for every C# test that
has no Rust test.

All of them build into one program, `core/tests/all`:

```
cargo test -p sobreiro-core
```

A C# `[Theory]` with rows is one Rust test that walks the same rows, so the Rust count is lower
than the count the C# test run reports. "Cases" below is that C# count: one for each `[Fact]` and
one for each row.

## Limits, usage and the files the bridge writes

Ported so far: the limits, `usage.json` (its format, reading and writing it, merging a reading
into it) and the atomic write. The bridge itself is the second piece of this area.

| C# test file | Rust file | C# cases | Ported | Rust tests |
| --- | --- | ---: | ---: | ---: |
| `LimitListTests.cs` | `all/limit_list.rs` | 28 | 28 | 14 |
| `UsageLevelTests.cs` | `all/usage_level.rs` | 8 | 8 | 3 |
| `RateLimitsJsonTests.cs` | `all/rate_limits_json.rs` | 20 | 20 | 14, and 4 new |
| `ResetPeriodTests.cs` | `all/reset_period.rs` | 4 | 4 | 4 |
| `UsageFileTests.cs` | `all/usage_file.rs` | 13 | 10 | 10, and 1 new |
| `UsageMergeTests.cs` | `all/usage_merge.rs` | 16 | 16 | 16 |
| `AtomicFileTests.cs` | `all/atomic_file.rs` | 6 | 6 | 6 |
| | | 95 | 92 | 67, and 5 new |

Still to port in this area, with the bridge: `BridgeRunnerTests.cs` (10 cases),
`BridgeConfigTests.cs` (6), `ChainRunnerTests.cs` (10) and `BashLocatorTests.cs` (6).

### C# tests with no Rust test yet

- `UsageFileTests.AppPaths_uses_SOBREIRO_DATA_DIR_when_set`,
  `UsageFileTests.AppPaths_ignores_relative_SOBREIRO_DATA_DIR` and
  `UsageFileTests.AppPaths_defaults_to_LocalApplicationData_when_unset`: they test `AppPaths`,
  not the usage file, and come over with the area "Paths and config".

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
