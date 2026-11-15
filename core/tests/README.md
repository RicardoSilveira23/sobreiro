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

The three `AppPaths_*` tests in `UsageFileTests.cs` are in `app_paths.rs`, before the tests of
`AppPathsTests.cs`, which came with the area "Paths and config" below.

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

## Paths and config

The whole area: where Sobreiro keeps its files and what a second copy is called, where Claude
Code keeps its own, and `config.json` (reading it, and writing it back to the byte as 1.x did).

| C# test file | Rust file | C# cases | Ported | Rust tests |
| --- | --- | ---: | ---: | ---: |
| `AppPathsTests.cs` | `core/tests/all/app_paths.rs` | 8 | 7 | 7, and 4 new |
| `AppConfigTests.cs` | `core/tests/all/app_config.rs` | 33 | 33 | 21, and 10 new |
| `ClaudePathsTests.cs` (Connect) | `core/tests/all/claude_paths.rs` | 5 | 5 | 3, and 2 new |
| | | 46 | 45 | 31, and 16 new |

### C# tests with no Rust test

- `AppPathsTests.Log_written_with_the_override_lands_in_the_scratch_folder` writes a line with
  the log, which is not ported yet. It comes with the log. That the log's path follows the
  override is held by `app_paths::log_file_lives_in_the_data_directory`.

### C# tests ported in part

- The tests of `AppPathsTests.cs` set `SOBREIRO_DATA_DIR` for the whole test run and ask
  Windows for the user's folder, and those of `ClaudePathsTests.cs` hand in a function standing
  in for the environment. In Rust the values themselves are handed to the code under test.
- `AppConfigTests.Corrupt_or_non_object_throws_instead_of_giving_default` and
  `A_file_that_cannot_be_read_throws_instead_of_giving_default` expect an exception of a given
  kind. A Rust function gives an error back instead: the tests hold which one.

### Rust tests with no C# test

What these hold was read off the C# of 1.1.0, run on 2026-11-14 in a data folder of its own.

- `app_paths::instance_id_is_the_one_the_csharp_gives`: 40 folders, spelled in many ways, each
  with the name the C# gave it. A copy of 2.0 and a copy of 1.x on the same folder must take
  the same name, or both could run on it at once.
- `app_paths::an_override_is_taken_when_it_names_drive_and_folder`: which values of
  `SOBREIRO_DATA_DIR` the C# takes and which it ignores. It takes some that name no folder
  (`\\server`); ignoring one would send a copy told to keep to itself into the user's own data.
- `app_paths::the_default_and_the_old_folder_do_not_follow_the_override` and
  `app_paths::the_data_folder_is_the_default_one_without_an_override_or_with_one_naming_it`:
  the three functions the first-run migration asks, which no C# test of this area names.
- `claude_paths::without_the_user_profile_variable_the_folder_windows_names_is_used` and
  `claude_paths::a_config_dir_that_starts_with_two_separators_is_taken`.
- `app_config::save_gives_the_bytes_the_csharp_writes`: three files the C# wrote, each written
  here from the same settings and found the same to the byte, then read and written again.
  This settles the line ends named in issue 5: a carriage return and a line feed, as 1.x.
- `app_config::a_command_is_escaped_as_the_csharp_escapes_it`: which characters of a command
  or of a key are written as a code and which as themselves.
- `app_config::save_refuses_a_kept_value_that_is_not_json`: the C# writer throws before
  anything touches the disk; here an error comes back and no file is written.
- `app_config::a_kept_value_is_written_as_it_was_read`: an unknown key's value is carried
  unread, with its spaces, its line ends, a number too large to hold and half a character.
- `app_config::a_key_written_twice`: which of the two counts, for each kind of key.
- `app_config::a_file_someone_is_writing_is_not_read`: the C# opens `config.json` so that
  nobody may write it meanwhile, and fails while a program has it open for writing. A
  half-written file is never taken for an empty one.
- `app_config::every_choice_is_written_by_its_name_and_read_back`: the nine names in the file.
- `app_config::keys_are_exact_and_names_ignore_only_capitals`.
- `app_config::load_reads_the_file_as_the_csharp_reads_it`: the mark at the start of a file,
  bytes that are no text, what counts as a blank file, and what a looser reader of JSON would
  let through.
- `app_config::half_a_character_where_sobreiro_reads_is_not_json`: the C# stops with an error
  of another kind; either way nothing is saved over the file.

## Connect and Disconnect

The whole area: reading and replacing the `statusLine` key of Claude Code's `settings.json`
with every other byte left alone, the plan to connect, disconnect or reconnect and carrying it
out with a backup, the command that runs the bridge, installing the bridge and finding it, and
the words of the flow.

| C# test file | Rust file | C# cases | Ported | Rust tests |
| --- | --- | ---: | ---: | ---: |
| `SettingsJsonTests.cs` | `core/tests/all/settings_json.rs` | 20 | 20 | 18, and 5 new |
| `ConnectionTests.cs` | `core/tests/all/connection.rs` | 28 | 28 | 26, and 3 new |
| `LegacyConnectionTests.cs` | `core/tests/all/legacy_connection.rs` | 18 | 18 | 12 |
| `BridgeCommandTests.cs` | `core/tests/all/bridge_command.rs` | 18 | 18 | 6, and 4 new |
| `BridgeInstallTests.cs` | `core/tests/all/bridge_install.rs` | 11 | 11 | 7, and 1 new |
| `BridgeLocatorTests.cs` | `core/tests/all/bridge_locator.rs` | 7 | 7 | 7 |
| `ConnectMessageTests.cs` | `core/tests/all/connect_message.rs` | 8 | 8 | 8, and 1 new |
| | `core/tests/all/csharp_files.rs` | | | 2 new |
| | | 110 | 110 | 84, and 16 new |

`ClaudePathsTests.cs`, also of this folder, came with "Paths and config" above.

### C# tests with no Rust test

None.

### C# tests ported in part

- Where a C# test expects an exception of a given kind (`FormatException`,
  `InvalidOperationException`), a Rust function gives an error back instead, and the test
  holds which one: 3 rows in `settings_json.rs`, 9 tests in `connection.rs`. The words of each
  error are the C#'s, and `settings_json::read_throws_format_exception_for_non_objects_and_bad_json`
  holds them.
- `ConnectionTests.A_settings_file_that_is_a_symbolic_link_stays_a_link` makes a symbolic
  link, which Windows lets a program do only with a privilege a developer's PC does not give
  by default. Without it the C# test ends early and passes, and so does the Rust one. CI has
  the privilege: there the Rust test fails if the link cannot be made, so it cannot pass by
  testing nothing.
- The tests of `ConnectionTests.cs` and `LegacyConnectionTests.cs` hand in the time as a
  moment with its distance from UTC. Only the date and time on the user's clock name the
  backup, so the Rust takes those.
- `LegacyConnectionTests.cs` builds its `settings.json` with the C# JSON writer, which
  writes a quote as `\u0022`. The Rust helper writes the same bytes by hand.
- `BridgeInstallTests.cs` hands `Ensure` a function that opens a stream; the Rust hands
  `ensure` a function that opens a reader. A reader does not say how long it is, so the
  Rust compares the bytes where the C# first compares the lengths. The answer is the same.

### Rust tests with no C# test

What these hold was read off the C# of 1.1.0, run on 2026-11-14 on files in a folder of its
own. The rows were written out by the C# itself and turned into Rust by a script, not typed.

- `csharp_files::connect_and_disconnect_leave_the_bytes_the_csharp_left`: 21 runs on 19
  sample `settings.json` files (no file, no status line, another tool's, Sobreiro's own, a
  Sobreiro bridge in another folder, the old `openheadroom-bridge`, odd spacing, comments
  and a comma after the last key, tabs, a byte order mark, both line ends and a mix of them,
  the key twice, no line end at the end, letters outside ASCII). Connect or Reconnect, then
  Disconnect: `settings.json`, `config.json` and both backups come out as the C# left them,
  to the byte.
- `csharp_files::connect_changes_nothing_outside_the_status_line_value`: the promise in its
  own words. On the same samples, every byte before the value and every byte after it is
  still there after Connect.
- `settings_json::read_status_line_answers_as_the_csharp_did`: 102 texts and what each is:
  a value, no value, not an object, or not JSON. Among them what counts as a blank file,
  where a comma may be, which numbers and texts are JSON, where a comment may sit, and the
  two line-ending characters the C# refuses inside a `//` comment.
- `settings_json::a_comment_may_sit_anywhere_but_between_a_name_and_its_colon`: the one
  place where the C# takes no comment.
- `settings_json::objects_and_lists_may_sit_64_deep`: the C# reader stops at 64.
- `settings_json::with_status_line_answers_as_the_csharp_did`: 35 replacements, removals
  and insertions with the text each gives: commas, comments, line ends and indentation
  around the key.
- `settings_json::command_of_answers_as_the_csharp_did`: 17 status lines and their command:
  the last of two, a name written with a code, a name in capitals.
- `bridge_command::status_line_json_answers_as_the_csharp_did`: 17 status lines written
  anew: nested values laid out again, numbers kept digit for digit, which characters are
  written as a code, a key named twice.
- `bridge_command::runs_answers_as_the_csharp_did`: 42 commands against a bridge's path:
  dots and spaces at the end, `..`, doubled separators, paths that start with `\\?\`, a
  share on another computer, and which letters are the same when capitals are ignored.
- `bridge_command::a_command_that_is_not_a_full_path_is_read_from_the_folder_the_program_runs_in`.
- `bridge_command::is_bridge_and_is_legacy_bridge_answer_as_the_csharp_did`: 13 commands.
- `connection::half_a_character_in_the_command_stops_connect_and_disconnect`,
  `connection::half_a_character_in_another_key_of_the_status_line_stops_connect` and
  `connection::half_a_character_outside_the_status_line_is_left_as_it_is`: JSON can spell
  out half of a character (`\ud800` with nothing after it). Where Sobreiro would have to
  read or rewrite it, the C# stops with an error of another kind; either way nothing is
  written. Elsewhere in the file it comes back as it was.
- `bridge_install::ensure_gives_up_on_a_program_that_really_runs_and_leaves_it_as_it_was`:
  the C# tests stand in for a running bridge with a file held open. This runs a real program
  from the file: the same bytes are still found to be the same, other bytes are refused
  after ten tries with "access denied", no temporary file is left, and once the program
  has ended the bridge is written.
- `connect_message::reconnect_has_its_own_words`: the title, the box after, and the line
  about the old bridge.

## Tokens and the HTTP clients: Claude Code's side

Claude Code's half of the area: its access token as Sobreiro holds it, reading it out of the
sign-in file, the one request made with it, and reading the reply. Copilot's half is the
section after this one.
Every token in these tests is made up, no test reads the real sign-in file, and no test
reaches further than a server on the same PC.

| C# test file | Rust file | C# cases | Ported | Rust tests |
| --- | --- | ---: | ---: | ---: |
| `ClaudeCredentialsTests.cs` | `core/tests/all/claude_credentials.rs` | 21 | 21 | 12, and 4 new |
| `LiveUsageClientTests.cs` | `core/tests/all/live_usage_client.rs`, `http.rs` | 20 | 19 | 17, and 7 new |
| `UsageApiJsonTests.cs` | `core/tests/all/usage_api_json.rs` | 14 | 14 | 8, and 5 new |
| | `core/tests/all/http.rs` | | | 15 new |
| | | 55 | 54 | 37, and 31 new |

Ported ahead of its own area, because its words came with this one:

| C# test file | Rust file | C# cases | Ported | Rust tests |
| --- | --- | ---: | ---: | ---: |
| `LiveStateTests.cs` (polling) | `core/tests/all/live_state.rs` | 1 of 9 | 1 | 1 |

The client is handed something that sends a request, as the C# client is handed a handler.
The tests of `live_usage_client.rs` hand it a stand-in. The tests of `http.rs` run the real
sender against a server on 127.0.0.1, written with the standard library.

### C# tests with no Rust test

- `LiveUsageClientTests.A_bad_charset_in_a_200_never_throws`: the C# framework turned a body
  into text by the character set its reply named, and stopped on a name it did not know. The
  Rust client is handed the body's bytes and no content type, and reads them as UTF-8
  whatever the reply calls them. `live_usage_client::a_body_is_read_as_utf_8` holds that.

### C# tests ported in part

- `A_200_gives_limits_and_sends_the_exact_request` asserts that the request is a GET. The
  seam has one way to ask, a GET, so there is nothing to assert there;
  `http::a_request_goes_out_as_it_was_asked_and_the_answer_comes_back` reads the word on the
  wire.
- `A_redirect_is_failed_and_not_followed` hands the C# client an answer that names another
  address. An answer as the Rust client is handed it has no place for that address. The
  test holds that a 302 is `Failed` and that one request was made.
- `Network_errors_and_timeouts_are_unreachable_without_the_token_in_the_text` and
  `A_non_network_exception_while_reading_the_body_is_failed_not_thrown` hand in errors whose
  words hold the token. A Rust sender gives back one of two names and no words, so the
  tests hold what each name becomes.
- `Cancelling_by_the_caller_propagates` expects an exception. The Rust client gives
  `Err(Cancelled)` back, and the test also holds that nothing was sent.
- `The_handler_neither_follows_redirects_nor_uses_cookies` reads two settings of the C#
  handler. The Rust tests show the two things themselves, on the wire:
  `http::a_redirect_comes_back_as_the_answer_and_is_not_followed` and
  `http::no_cookie_is_kept_from_one_request_to_the_next`.
- `A_reply_slower_than_the_timeout_is_unreachable` gives the C# client a short timeout and
  a slow stand-in. In Rust the sender keeps the time:
  `http::an_answer_slower_than_the_timeout_is_unreachable` runs the real one against a server
  that never answers, and `network_errors_and_timeouts_are_unreachable_without_the_token_in_the_text`
  holds what the client makes of it.
- `ClaudeCredentialsTests.Token_never_appears_in_ToString_or_results` holds the one way a C#
  value is printed. Rust has two, and a long form of the second: the test holds all three.
- The tests of `ClaudeCredentialsTests.cs` read a status and a token that may be missing.
  In Rust only `Ok` carries a token, so "no token" needs no assertion of its own.

### Rust tests with no C# test

What the rows hold was read off the C# of 1.1.0, run on 2026-11-14 on made-up files and
made-up replies, with a stand-in for the network. The rows were written out by the C# and
turned into Rust by a script, not typed.

- `claude_credentials::read_answers_as_the_csharp_did`: 103 sign-in files and what each gives.
  Among them which numbers are an expiry, the first and the last moment that is one, a key
  written twice or with codes, what is not plain JSON, how deep a file may nest, the mark at
  the start of a file, and bytes that are no text outside the token.
- `claude_credentials::a_token_that_is_no_text_is_unreadable`: half a character spelled out
  in the token, or bytes in it that are not UTF-8. The C# stopped with an error of another
  kind; either way no token comes out.
- `claude_credentials::a_megabyte_is_read_and_one_byte_more_is_not`.
- `claude_credentials::a_missing_folder_is_missing_and_a_name_no_file_can_have_is_unreadable`:
  which errors of Windows count as "not there". The C# stopped with an error on a path of
  nothing; here that is `Missing`.
- `usage_api_json::resets_at_answers_as_the_csharp_did`: 118 ways to write a reset time and
  the moment each names, to the ten-millionth of a second, or none.
- `usage_api_json::a_time_that_does_not_say_how_far_it_is_from_utc_is_no_time`: the C# took
  such a time to be on the PC's own clock. Core does not know the PC's time zone and reads
  no time.
- `usage_api_json::percent_answers_as_the_csharp_did`: 40 ways to write a percentage. A
  number too large to hold is infinity, as in the C#, and a zero keeps its minus sign.
- `usage_api_json::parse_answers_as_the_csharp_did`: 70 replies: which entries of the list
  count, the scope, a key written twice, the keys and the list together, and what is not
  plain JSON.
- `usage_api_json::half_a_character_where_a_text_is_read_makes_the_reply_no_use`: the C#
  stopped with an error, which its caller took for a failed request.
- `live_usage_client::every_status_answers_as_the_csharp_did`: 84 statuses.
- `live_usage_client::retry_after_answers_as_the_csharp_did`: 149 ways to write
  `Retry-After`, as seconds or as a date, and the wait each gives.
- `live_usage_client::the_request_goes_to_the_one_address_whatever_the_answer`.
- `live_usage_client::cancelling_while_the_request_is_under_way_drops_the_answer`.
- `live_usage_client::a_token_or_an_app_name_that_no_header_can_hold_is_not_sent` and
  `a_token_with_spaces_and_signs_is_sent_as_it_is`: a line end or a zero in the token gave
  `Failed` in the C# too. A tab, a letter outside ASCII and an empty token the C# handed on
  to its framework; here they are not sent.
- `live_usage_client::a_body_is_read_as_utf_8`: the mark at the start, bytes that are no
  text, and a body in UTF-16, which the C# read and this does not.
- `http.rs`, 15 of its 18 tests: the request on the wire with no header added, a redirect to
  the same server and to another one, statuses of 400 and more, `Retry-After` as written, an
  answer that stops half way or is not HTTP, a body that stalls, nobody listening, the
  largest body, an address or a header that cannot be, and the app's ten seconds. Three hold
  a rule each:
  - `one_file_of_the_workspace_reaches_the_network_and_no_other` fails when a second file of
    Core, of the bridge or of the shell names the HTTP library or a network type.
  - `plain_http_is_spoken_to_this_pc_and_to_nowhere_else`: outside 127.0.0.1 a request is
    HTTPS or it is not made.
  - `a_logger_set_to_hear_everything_is_not_told_the_token`: the HTTP library can write a
    request out byte by byte to a logger at its most detailed level. No logger is installed,
    and that level is compiled out of the build; the test installs one and listens.

## Tokens and the HTTP clients: Copilot's side

Copilot's half of the area: a GitHub token as Sobreiro holds it, finding one in a text, in
the bytes of a sign-in entry and in the environment, the one request made with it, and
reading the reply. Looking the bytes up in Windows, asking the GitHub CLI, the poll and the
CLI's cache file belong to the area "Copilot: its provider, its sign-in and the CLI's cache
file". Every token in these tests is made up, no test reads the real environment or a real
sign-in, and no test reaches the network: the client is handed the stand-in that Claude
Code's client is handed, which now lives in `main.rs`.

| C# test file | Rust file | C# cases | Ported | Rust tests |
| --- | --- | ---: | ---: | ---: |
| `CopilotTokenTests.cs` | `core/tests/all/copilot_token.rs` | 14 | 13 | 13, and 7 new |
| `CopilotUsageClientTests.cs` | `core/tests/all/copilot_usage_client.rs` | 16 | 16 | 11, and 10 new |
| `CopilotUserJsonTests.cs` | `core/tests/all/copilot_user_json.rs` | 20 | 20 | 16, and 3 new |
| | `core/tests/all/http.rs` | | | 1 new |
| | | 50 | 49 | 40, and 21 new |

The C# reader gives a reading or nothing, and the C# client then asks a second question to
tell two kinds of nothing apart: is the body a JSON object at all. The Rust reader answers
both at once, with a reading, `NoBudget` or `NoUse`, so the tests ported from
`CopilotUserJsonTests.cs` hold which of the two a reply without a reading is.

### C# tests with no Rust test

- `CopilotTokenTests.Text_already_in_the_buffer_is_never_searched`: the C# decodes a sign-in
  entry into a buffer it borrows, which can still hold someone else's old text after the
  part just decoded. The Rust searches the bytes where they lie and decodes nothing, so
  there is no buffer to hold old text.

### C# tests ported in part

- `The_decoded_text_is_cleared_and_only_the_token_is_kept`,
  `The_decoded_text_is_cleared_when_it_holds_no_token`,
  `A_utf16_token_is_found_after_the_utf8_pass_and_the_text_is_cleared` and
  `Odd_contents_fit_the_buffer_and_are_cleared` also hold that the buffer the entry was
  decoded into is wiped. There is no such buffer here. The tests hold what comes out:
  `only_the_token_is_kept_of_what_a_blob_holds`, `a_blob_that_holds_no_token_gives_none`,
  `a_utf16_token_is_found_after_the_utf8_pass` and `odd_contents_give_the_token_or_none`.
- `Text_without_a_token_gives_null` also hands in no text at all. In Rust a text is always
  there; the test hands in an empty one.
- The tests of `CopilotTokenTests.cs` compare tokens as texts. A Rust token can be neither
  compared nor printed, so each test takes its value out first.
- `Sends_the_four_headers_and_nothing_else` asserts that the request is a GET. The seam has
  one way to ask, a GET; `http::a_request_goes_out_as_it_was_asked_and_the_answer_comes_back`
  reads the word on the wire.
- `Redirect_is_Failed` hands the C# client an answer that names another address. An answer
  as the Rust client is handed it has no place for that address. The test holds that a 302
  is `Failed` and that one request was made.
- `Network_error_is_Unreachable` hands in an error whose words hold the token, and
  `Timeout_is_Unreachable` a reply that takes too long. A Rust sender gives back one of two
  names and no words, and keeps the time itself, so both are one test:
  `a_network_error_or_a_timeout_is_unreachable`. That the real sender gives up in time is
  `http::an_answer_slower_than_the_timeout_is_unreachable`.
- `Caller_cancellation_throws` expects an exception. The Rust client gives `Err(Cancelled)`
  back, and the test also holds that nothing was sent.
- `Token_never_appears_in_the_result` holds the one way a C# value is printed. A Rust result
  has one way and a long form of it: the test holds both.

### Rust tests with no C# test

What the rows hold was read off the C# of 1.1.0, run on 2026-11-14 on made-up tokens and
made-up replies, with a stand-in for the network. The rows were written out by the C# and
turned into Rust by a script, not typed.

- `copilot_token::a_token_is_never_printed`: neither way of printing shows the token, nor
  does a larger value that holds one.
- `copilot_token::find_answers_as_the_csharp_did`: 91 texts and the token found in each.
  Among them the four starts and what is not one, capitals, where a token ends, a start with
  nothing after it, two tokens in one text, and letters and digits outside ASCII.
  `a_long_text_is_searched_to_its_end` holds two more, of five thousand characters.
- `copilot_token::from_blob_answers_as_the_csharp_did`: 57 blobs. Among them UTF-16 that
  starts on an odd byte or has the high byte first, one token in each encoding, bytes that
  are no text before, inside and after a token, and a letter of UTF-16 whose high byte is
  not zero.
- `copilot_token::from_environment_answers_as_the_csharp_did`: 29 environments, and
  `the_characters_taken_off_the_ends_of_a_value_are_the_ones_the_csharp_took_off`: every
  character but the halves, 63,488 of them, at both ends of a value. The C# took off 25.
- `copilot_token::the_environment_is_asked_for_three_names_and_no_other`.
- `copilot_user_json::parse_answers_as_the_csharp_did`: 231 replies. Among them which of the
  two objects counts, a key written twice or with codes, what counts as unlimited, which
  numbers are a budget, the four fields that say what is used and their order, what is over
  the budget, the moment of reading, a reset exactly 400 days off and one ten-millionth of a
  second further, what is not plain JSON, and how deep a reply may nest. The numbers are held
  bit for bit.
- `copilot_user_json::half_a_character_where_a_time_is_read_makes_the_reply_no_use`: the C#
  reader stopped with an error, which its client took for a failed request.
- `copilot_user_json::a_time_that_does_not_say_how_far_it_is_from_utc_is_no_time`: the C#
  took such a time to be on the PC's own clock. Core does not know the PC's time zone and
  reads no time, as in Claude Code's reply.
- `copilot_usage_client::every_status_answers_as_the_csharp_did`: 84 statuses, each with a
  business seat's body, a free plan's and a list.
- `copilot_usage_client::every_body_answers_as_the_csharp_did`: 41 bodies. Among them the
  mark at the start, bytes that are no text, what follows an object, and half a character
  where it is read and where it is not.
- `copilot_usage_client::a_body_in_utf_16_is_not_read`: the C# read one that began with its
  mark, and this does not. The C# also read a body by the character set its reply named and
  gave `Failed` for a name it did not know; the Rust client is handed the bytes and no
  content type, and reads them as UTF-8.
- `copilot_usage_client::retry_after_answers_as_the_csharp_did`: 16 ways to write
  `Retry-After`. The header is read by the function Claude Code's client uses, which has its
  149 rows. `retry_after_is_read_off_a_429_and_off_no_other_status`: a 403 that says how
  long to wait is a refusal like any other, as in the C#.
- `copilot_usage_client::the_request_goes_to_the_one_address_whatever_the_answer`.
- `copilot_usage_client::a_sender_that_could_not_make_the_request_is_failed`.
- `copilot_usage_client::cancelling_while_the_request_is_under_way_drops_the_answer`.
- `copilot_usage_client::a_token_or_an_app_name_that_no_header_can_hold_is_not_sent` and
  `a_token_with_spaces_and_signs_is_sent_as_it_is`: a line end or a zero in the token gave
  `Failed` in the C# too. A tab, a letter outside ASCII and an empty token the C# handed on
  to its framework, and an app name it handed on whatever was in it; here they are not sent.
- `http::two_addresses_are_written_in_the_code_and_no_other` holds a rule: each token goes to
  one address, and each address is a constant of its client. It fails when a line of code in
  Core, in the bridge or in the shell holds any other address.

## Polling, back-off and the live state for Claude Code

When the next poll for Claude Code's usage is due and what each outcome does to that
(`LiveSchedule`), one poll (`LivePoll`), the sentence for the foot of the flyout
(`LiveState`), Claude Code as a provider that keeps its readings in `usage.json`
(`ClaudeProvider`), and the folder `Polling`: what a provider is, the states it can be in, and
the timing of its polls (`ProviderPoller`). Copilot's provider and its poll belong to the
area "Copilot: its provider, its sign-in and the CLI's cache file".

Nothing in the area looks at a clock, waits for a poll or starts a thread. Every test hands
in the time, a made-up token, a sign-in file and a `usage.json` of its own folder, and a
stand-in for the network.

| C# test file | Rust file | C# cases | Ported | Rust tests |
| --- | --- | ---: | ---: | ---: |
| `ClaudeProviderTests.cs` | `core/tests/all/claude_provider.rs` | 10 | 10 | 10, and 6 new |
| `LivePollTests.cs` | `core/tests/all/live_poll.rs` | 20 | 19 | 14, and 7 new |
| `LiveScheduleTests.cs` | `core/tests/all/live_schedule.rs` | 30 | 30 | 23, and 10 new |
| `LiveStateTests.cs` | `core/tests/all/live_state.rs` | 9 | 9 | 3 |
| `ProviderPollerTests.cs` | `core/tests/all/provider_poller.rs` | 16 | 16 | 15, and 6 new |
| | | 85 | 84 | 65, and 29 new |

One of the nine cases of `LiveStateTests.cs`, the About line, came with the tokens and is
counted there too.

The C# poller is handed three things to call: one that runs a poll, one that redraws and one
that traces, and a poll that has run is handed back through the run itself. The Rust poller
calls nobody. `start`, `tick` and `flyout_opened` give back the poll to run, when there is
one, and `done` gives back whether to redraw, the line to trace and sometimes the next poll.
So where a C# test counts calls, the Rust test counts what came back.

### C# tests with no Rust test

- `LivePollTests.An_ok_read_without_a_token_sends_nothing`: a C# read of the sign-in file is
  a status and a token that may be missing, so "all well" with no token can be written down.
  A Rust read that is `Ok` is the token: there is no such value to hand in.

### C# tests ported in part

- `LiveScheduleTests.A_refusal_needs_the_expiry_so_after_fetch_turns_it_away` expects an
  exception. `after_fetch` gives `Err(RefusalNeedsItsExpiry)` back.
- `LivePollTests.A_request_timing_out_is_offline` and `An_unreachable_host_is_offline` hand
  the C# client two kinds of error. A Rust sender gives one name for both, so they are one
  test: `an_unreachable_host_or_a_request_timing_out_is_offline`.
- `LivePollTests.The_callers_cancellation_leaves_as_a_cancellation` and
  `ClaudeProviderTests.Cancellation_passes_through` expect an exception. The Rust gives
  `Err(Cancelled)` back, and the first also holds that nothing was sent.
- `LivePollTests.The_token_never_appears_in_a_result` holds the one way a C# value is
  printed. A Rust result has one way and a long form of it: the test holds both.
- `ClaudeProviderTests.A_poll_passes_on_the_schedule_the_state_and_the_outcome` asserts that
  the schedule handed on is the very same object. A Rust schedule is a value that is copied;
  the test hands in a schedule unlike the one the poll started with and holds that this one
  comes out.
- `ClaudeProviderTests.A_held_usage_json_skips_the_reading` asserts that the trace holds the
  line; the Rust test, that the trace is that line and no other.
- `ProviderPollerTests.A_poll_that_threw_counts_as_a_bad_answer` and
  `A_cancelled_task_without_stop_counts_as_a_bad_answer`: a C# poll can end with an error of
  any kind, and its type's name goes into the trace. A Rust poll gives an answer or
  `Cancelled`, with no third way to end, so both are one test:
  `a_poll_given_up_without_a_stop_counts_as_a_bad_answer`. The line traced is
  "Fake poll was cancelled" where the C# said "Fake poll threw nothing".
- `ProviderPollerTests.An_answer_sets_the_state_keeps_the_reading_and_redraws` reads the
  state from inside the call that redraws, and holds that the reading was kept before. The
  Rust poller says "redraw" in what `done` gives back, by when both have happened: the test
  holds the state, the misses and the kept reading once `done` has returned.
- `ProviderPollerTests.Stop_cancels_the_poll_and_turns_it_off` looks at the token of a poll
  that has ended. The Rust test stops the poller while the poll is under way on another
  thread, as the app does, and the poll itself notes that it was given up.
- `ProviderPollerTests.A_run_handed_back_twice_does_nothing_the_second_time`: handing a Rust
  answer back gives it away, so the compiler refuses a second time. What is left to go wrong
  is an answer handed to a poller that did not start its poll:
  `an_answer_from_another_pollers_poll_does_nothing`.
- `ProviderPollerTests.After_dispose_nothing_happens`: a Rust poller that is no longer wanted
  is dropped, and nothing can be asked of it afterwards. The test holds what is left:
  `dropping_the_poller_cancels_the_poll_under_way`.

### Rust tests with no C# test

What the rows hold was read off the C# of 1.1.0, run on 2026-11-14 on made-up clocks,
made-up sign-in files and made-up replies, with a stand-in for the network. The rows were
written out by the C# and turned into Rust by a script, not typed. A moment in a row is a
count of ticks, the ten-millionths of a second the C# clock counts in.

- `live_schedule::every_event_answers_as_the_csharp_did`: 375 rows, twelve schedules times
  everything that can happen to one. Among them a 429 at every step of the back-off and
  past its end, with no `Retry-After`, with one shorter than the step, as long and one tick
  longer, and with six hours and seven days; a wait handed in with an ok and with a failure, which is
  not looked at; a refusal with and without an expiry; and the same a day earlier, for a
  clock that was set back.
- `live_schedule::several_events_in_a_row_answer_as_the_csharp_did`: nine walks, 47 steps.
  Six 429s in a row with and without a wait asked for, 429s with a failure and a success
  between, five failures, a refusal that lasts through three readings of the file, and two
  walks where the clock goes back in the middle.
- `live_schedule::is_due_and_fetch_on_open_answer_as_the_csharp_did`: 187 rows. A tick
  before, at and after each moment a schedule holds, a day and an hour before (the clock
  was set back), and ten years on.
- `live_schedule::refuses_answers_as_the_csharp_did`: 35 rows.
- `live_schedule::a_clock_set_back_waits_for_the_old_moment` and
  `a_clock_set_forward_is_due_at_once`: the test of a clock change. A schedule holds moments of the PC's clock. Set back, no poll is due until the
  old moment comes round, and opening the flyout does not ask sooner; set forward, a poll
  is due at once. 1.x does the same, and the rows above hold it.
- `live_schedule::the_counts_stop_at_the_top_and_never_go_round` and
  `a_moment_past_the_end_of_the_calendar_is_the_calendars_last`: where the C# count went
  round to a number below zero, and where a moment past the year 9999 stopped it with an
  error. Neither can be reached by a clock that is right.
- `live_schedule::a_failure_goes_through_after_failure_as_through_after_fetch`: the poller
  needs "a request failed" without making up a result.
- `live_schedule::a_schedule_holds_moments_and_counts_and_prints_them`: a schedule is what
  is kept from one poll to the next, and nothing in it could be a token.
- `live_poll::each_poll_reads_the_sign_in_file_again_and_sends_what_it_holds_then` holds
  "per request": one client, three polls, the file changed and then deleted between them.
- `live_poll::a_sign_in_that_ran_out_or_was_refused_is_neither_sent_nor_touched` holds
  "never renew": no request for either, through 28 more polls, the file byte for byte as
  it was, and nothing new beside it.
- `live_poll::a_token_handed_in_by_the_reader_goes_to_the_request_only`.
- `live_poll::cancelling_while_the_request_is_under_way_drops_the_answer` and
  `a_cancelled_poll_with_no_sign_in_to_send_still_answers`.
- `live_poll::a_sender_that_could_not_make_the_request_is_a_bad_answer`.
- `live_poll::every_outcome_has_the_name_the_csharp_logged`.
- `claude_provider::a_poll_through_the_provider_answers_as_the_csharp_did`: 122 polls. Ten
  sign-in files (missing, not JSON, no token, an empty one, run out a minute ago, running
  out now and in a millisecond, good, without an expiry), five schedules (fresh, refusing
  a sign-in with and without an expiry, in the back-off, after a failure) and sixteen
  replies. Each row holds how many requests were sent, the state, the line for the log,
  the next schedule and whether there is a reading to keep.
- `claude_provider::keep_leaves_usage_json_as_the_csharp_left_it`: 21 rows, seven files and
  three replies. `usage.json` afterwards is compared byte for byte, and the limits before
  and after number for number.
- `claude_provider::a_kept_reading_carries_the_time_it_was_kept_at`.
- `claude_provider::a_usage_json_that_cannot_be_written_is_traced_without_its_path`: the
  trace is told the kind of the failure in one word, where the C# gave the name of the
  exception's type. Neither gives the error's own words or a path.
- `claude_provider::a_provider_without_a_trace_skips_a_held_usage_json_in_silence` and
  `the_poll_is_handed_what_the_provider_was_handed`.
- `provider_poller::walks_of_calls_answer_as_the_csharp_did`: twelve walks of forty calls
  (start, tick, open, stop, the poll itself, its answer handed back), picked by a dice with
  a fixed seed in the C#, with seven kinds of answer and a clock that now and then goes
  back an hour. After every call the row holds how many polls were started, the state, the
  misses, the redraws, the readings kept and when, and the lines traced.
- `provider_poller::after_a_stop_and_a_start_the_state_is_off_until_an_answer`: what 1.x
  does, kept as it is.
- `provider_poller::opening_the_flyout_starts_a_poll_that_is_due_whatever_else_holds`: the
  poller starts a poll on an open for two reasons, and no schedule the C# makes tells them
  apart. The test hands the poller one that does.
- `provider_poller::stop_before_any_start_turns_it_off`,
  `a_stopped_poll_that_gave_up_is_dropped_too` and `the_poller_gives_its_provider`.
