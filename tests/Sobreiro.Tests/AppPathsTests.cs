using Sobreiro.Core;
using Sobreiro.Core.Diagnostics;

namespace Sobreiro.Tests;

[Collection("env")]
public class AppPathsTests
{
    private const string Variable = "SOBREIRO_DATA_DIR";

    private static string Id(string? dir)
    {
        var old = Environment.GetEnvironmentVariable(Variable);
        try
        {
            Environment.SetEnvironmentVariable(Variable, dir);
            return AppPaths.InstanceId();
        }
        finally
        {
            Environment.SetEnvironmentVariable(Variable, old);
        }
    }

    [Fact]
    public void InstanceId_without_override_is_plain() => Assert.Equal("Sobreiro", Id(null));

    [Fact]
    public void InstanceId_with_override_is_a_hash_that_ignores_case()
    {
        var a = Id(@"C:\Temp\Scratch");
        Assert.Matches("^Sobreiro\\.[0-9a-f]{8}$", a);
        Assert.Equal(a, Id(@"c:\temp\SCRATCH"));
        Assert.NotEqual(a, Id(@"C:\Temp\Other"));
    }

    [Fact]
    public void InstanceId_ignores_a_trailing_separator() =>
        Assert.Equal(Id(@"C:\Temp\Scratch"), Id(@"C:\Temp\Scratch\"));

    [Fact]
    public void InstanceId_with_an_override_naming_the_default_folder_is_plain()
    {
        var local = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        Assert.Equal("Sobreiro", Id(Path.Combine(local, "Sobreiro")));
        Assert.Equal("Sobreiro", Id(Path.Combine(local, "SOBREIRO") + Path.DirectorySeparatorChar));
        Assert.Equal("Sobreiro", Id(Path.Combine(local.ToLowerInvariant(), "sobreiro")));
    }

    [Fact]
    public void AlertsFile_is_alerts_json() => Assert.EndsWith("alerts.json", AppPaths.AlertsFile());

    private static void WithDataDir(string dir, Action body)
    {
        var old = Environment.GetEnvironmentVariable(Variable);
        try
        {
            Environment.SetEnvironmentVariable(Variable, dir);
            body();
        }
        finally
        {
            Environment.SetEnvironmentVariable(Variable, old);
        }
    }

    [Fact]
    public void LogFile_lives_in_the_data_directory()
    {
        var dir = Path.Combine(Path.GetTempPath(), "sobreiro-tests-" + Guid.NewGuid().ToString("N"));
        WithDataDir(dir, () => Assert.Equal(Path.Combine(dir, "log.txt"), AppPaths.LogFile()));
    }

    [Fact]
    public void Log_written_with_the_override_lands_in_the_scratch_folder()
    {
        var dir = Path.Combine(Path.GetTempPath(), "sobreiro-tests-" + Guid.NewGuid().ToString("N"));
        try
        {
            WithDataDir(dir, () =>
                Assert.True(new DiagnosticsLog(AppPaths.LogFile()).Write("start version=test instance=x")));
            Assert.Contains("start version=test instance=x", File.ReadAllText(Path.Combine(dir, "log.txt")));
        }
        finally
        {
            if (Directory.Exists(dir)) Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public void BridgeFile_is_in_the_data_folder() =>
        Assert.Equal(Path.Combine(AppPaths.DataDirectory(), "sobreiro-bridge.exe"), AppPaths.BridgeFile());
}
