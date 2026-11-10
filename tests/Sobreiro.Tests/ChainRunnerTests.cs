using Sobreiro.Core.Bridge;

namespace Sobreiro.Tests;

public class ChainRunnerTests
{
    [Fact]
    public void StartInfo_uses_bash_dash_c()
    {
        var psi = ChainRunner.StartInfo("echo hi", @"C:\Git\bin\bash.exe");
        Assert.Equal(@"C:\Git\bin\bash.exe", psi.FileName);
        Assert.Equal(["-c", "echo hi"], psi.ArgumentList);
        Assert.True(psi.RedirectStandardInput);
        Assert.True(psi.RedirectStandardOutput);
        Assert.False(psi.UseShellExecute);
    }

    [Fact]
    public void StartInfo_falls_back_to_powershell()
    {
        var psi = ChainRunner.StartInfo("echo hi", null);
        Assert.Equal("powershell.exe", psi.FileName);
        Assert.Equal(["-NoProfile", "-NonInteractive", "-Command", "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); echo hi"], psi.ArgumentList);
    }

    [Theory]
    [InlineData(@"C:\Git\bin\bash.exe")]
    [InlineData(null)]
    public void StartInfo_marks_the_child_as_inside_a_chain(string? bashPath)
    {
        var psi = ChainRunner.StartInfo("echo hi", bashPath);
        Assert.Equal("1", psi.Environment["SOBREIRO_IN_CHAIN"]);
    }

    [Fact]
    public void Run_passes_stdin_and_returns_stdout_and_exit_code()
    {
        var bash = BashLocator.Find(Environment.GetEnvironmentVariable, File.Exists);
        if (bash is null) return; // no Git Bash on this machine: nothing to run against
        var result = ChainRunner.Run("cat; exit 3", "hello", bash);
        Assert.Equal(new ChainResult(3, "hello"), result);
    }

    [Fact]
    public void Run_returns_null_when_executable_missing() =>
        Assert.Null(ChainRunner.Run("echo hi", "", @"C:\nope\bash.exe"));

    [Fact]
    public void Run_times_out_and_returns_null()
    {
        var bash = BashLocator.Find(Environment.GetEnvironmentVariable, File.Exists);
        if (bash is null) return;
        var watch = System.Diagnostics.Stopwatch.StartNew();
        var result = ChainRunner.Run("sleep 20", "", bash, TimeSpan.FromMilliseconds(500));
        Assert.Null(result);
        Assert.True(watch.Elapsed < TimeSpan.FromSeconds(5), $"took {watch.Elapsed}");
    }

    [Fact]
    public void Run_powershell_output_is_utf8()
    {
        // This checks the encoding, not the speed: a cold PowerShell on a CI runner can take
        // longer than the default 5 s to start, so give it room.
        var result = ChainRunner.Run("Write-Output '5h · 7d'", "", null, TimeSpan.FromSeconds(30));
        Assert.Equal("5h · 7d", result!.Output.TrimEnd());
    }

    [Fact]
    public void Run_times_out_when_the_child_never_reads_a_large_stdin()
    {
        var bash = BashLocator.Find(Environment.GetEnvironmentVariable, File.Exists);
        if (bash is null) return;
        var watch = System.Diagnostics.Stopwatch.StartNew();
        var result = ChainRunner.Run("sleep 20", new string('x', 64 * 1024), bash, TimeSpan.FromMilliseconds(500));
        Assert.Null(result);
        Assert.True(watch.Elapsed < TimeSpan.FromSeconds(5), $"took {watch.Elapsed}");
    }

    [Fact]
    public void Run_times_out_when_a_background_child_keeps_stdout_open()
    {
        var bash = BashLocator.Find(Environment.GetEnvironmentVariable, File.Exists);
        if (bash is null) return;
        var watch = System.Diagnostics.Stopwatch.StartNew();
        var result = ChainRunner.Run("sleep 20 & echo hi", "", bash, TimeSpan.FromMilliseconds(1000));
        Assert.Null(result);
        Assert.True(watch.Elapsed < TimeSpan.FromSeconds(5), $"took {watch.Elapsed}");
    }
}
