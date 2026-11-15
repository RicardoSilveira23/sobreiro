using System.Diagnostics;
using Sobreiro.Core.Copilot;

namespace Sobreiro.Tests;

public class GhCliTests
{
    private static readonly string Fake = "gho_" + new string('a', 36);
    private static readonly TimeSpan Five = TimeSpan.FromSeconds(5);

    [Fact]
    public void PATH_lookup_finds_gh_exe_in_an_absolute_entry_and_skips_empty_and_relative_ones()
    {
        var dir = Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);
        try
        {
            var exe = Path.Combine(dir, "gh.exe");
            File.WriteAllText(exe, "");
            Assert.Equal(exe, GhCli.Find(string.Join(Path.PathSeparator, "", "relative", dir)));
            Assert.Null(GhCli.Find(string.Join(Path.PathSeparator, "", "relative")));
            Assert.Null(GhCli.Find(Path.Combine(dir, "missing")));
            Assert.Null(GhCli.Find(null));
        }
        finally
        {
            Directory.Delete(dir, true);
        }
    }

    [Fact]
    public void Missing_exe_gives_no_token() =>
        Assert.Empty(GhCli.Run(@"C:\nope\gh-missing.exe", "auth token", Five));

    [Fact]
    public void Printed_token_is_returned() =>
        Assert.Equal([Fake], GhCli.Run("cmd.exe", $"/c echo {Fake}", Five));

    [Fact]
    public void Non_zero_exit_gives_no_token() =>
        Assert.Empty(GhCli.Run("cmd.exe", $"/c echo {Fake} & exit 1", Five));

    [Fact]
    public void Grandchild_holding_the_pipe_open_does_not_stall_the_read()
    {
        var watch = Stopwatch.StartNew();
        Assert.Empty(GhCli.Run("cmd.exe", $"/c \"start /b ping -n 30 127.0.0.1 & echo {Fake}\"", TimeSpan.FromSeconds(1)));
        Assert.True(watch.Elapsed < TimeSpan.FromSeconds(4), $"took {watch.Elapsed}");
    }

    [Fact]
    public void Hanging_exe_is_killed_after_the_timeout()
    {
        var watch = Stopwatch.StartNew();
        Assert.Empty(GhCli.Run("ping.exe", "-n 30 127.0.0.1", TimeSpan.FromSeconds(1)));
        Assert.True(watch.Elapsed < TimeSpan.FromSeconds(4), $"took {watch.Elapsed}");
    }
}
