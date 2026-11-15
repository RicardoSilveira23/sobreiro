using Sobreiro.Core.Connect;

namespace Sobreiro.Tests;

public class BridgeInstallTests : IDisposable
{
    private static readonly byte[] Bytes = Enumerable.Range(0, 5000).Select(i => (byte)(i * 7)).ToArray();

    private readonly string _dir = Path.Combine(Path.GetTempPath(), "sobreiro-tests-" + Guid.NewGuid().ToString("N"));
    private string BridgePath => Path.Combine(_dir, "data", "sobreiro-bridge.exe");

    public void Dispose()
    {
        if (Directory.Exists(_dir)) Directory.Delete(_dir, recursive: true);
    }

    private static byte[] Flipped()
    {
        var other = (byte[])Bytes.Clone();
        other[2500] ^= 0xFF;
        return other;
    }

    [Fact]
    public void Ensure_writes_the_bridge_and_its_folder_when_missing()
    {
        Assert.True(BridgeInstall.Ensure(() => new MemoryStream(Bytes), BridgePath));
        Assert.Equal(Bytes, File.ReadAllBytes(BridgePath));
    }

    [Fact]
    public void Ensure_leaves_an_identical_bridge_alone()
    {
        Directory.CreateDirectory(Path.GetDirectoryName(BridgePath)!);
        File.WriteAllBytes(BridgePath, Bytes);
        var old = new DateTime(2020, 1, 1, 0, 0, 0, DateTimeKind.Utc);
        File.SetLastWriteTimeUtc(BridgePath, old);

        Assert.False(BridgeInstall.Ensure(() => new MemoryStream(Bytes), BridgePath));
        Assert.Equal(old, File.GetLastWriteTimeUtc(BridgePath));
    }

    [Fact]
    public void Ensure_replaces_other_bytes_of_the_same_length()
    {
        Directory.CreateDirectory(Path.GetDirectoryName(BridgePath)!);
        File.WriteAllBytes(BridgePath, Flipped());

        Assert.True(BridgeInstall.Ensure(() => new MemoryStream(Bytes), BridgePath));
        Assert.Equal(Bytes, File.ReadAllBytes(BridgePath));
    }

    [Fact]
    public void Ensure_waits_for_a_bridge_that_is_running()
    {
        Directory.CreateDirectory(Path.GetDirectoryName(BridgePath)!);
        File.WriteAllBytes(BridgePath, Flipped());
        var running = new FileStream(BridgePath, FileMode.Open, FileAccess.Read, FileShare.Read);
        var release = new Thread(() =>
        {
            Thread.Sleep(250);
            running.Dispose();
        });
        release.Start();

        Assert.True(BridgeInstall.Ensure(() => new MemoryStream(Bytes), BridgePath));
        release.Join();
        Assert.Equal(Bytes, File.ReadAllBytes(BridgePath));
    }

    [Fact]
    public void Ensure_throws_and_leaves_no_temp_file_when_the_bridge_stays_locked()
    {
        Directory.CreateDirectory(Path.GetDirectoryName(BridgePath)!);
        var other = Flipped();
        File.WriteAllBytes(BridgePath, other);
        using var running = new FileStream(BridgePath, FileMode.Open, FileAccess.Read, FileShare.Read);

        var error = Record.Exception(() => BridgeInstall.Ensure(() => new MemoryStream(Bytes), BridgePath));

        Assert.True(error is IOException or UnauthorizedAccessException, error?.ToString());
        Assert.Equal(new[] { BridgePath }, Directory.GetFiles(Path.GetDirectoryName(BridgePath)!));
        running.Dispose();
        Assert.Equal(other, File.ReadAllBytes(BridgePath));
    }

    [Theory]
    [InlineData("{\"statusLine\":{\"type\":\"command\",\"command\":\"\\\"C:/Data/sobreiro-bridge.exe\\\"\"}}", true)]
    [InlineData("{\"statusLine\":{\"type\":\"command\",\"command\":\"\\\"C:/Old/sobreiro-bridge.exe\\\"\"}}", false)]
    [InlineData("{}", false)]
    [InlineData("not json", false)]
    [InlineData("", false)]
    public void ShouldRefresh_only_when_the_status_line_runs_the_installed_copy(string settings, bool expected) =>
        Assert.Equal(expected, BridgeInstall.ShouldRefresh(settings, @"C:\Data\sobreiro-bridge.exe"));

    [Fact]
    public void ShouldRefresh_when_the_installed_path_has_forward_slashes() =>
        Assert.True(BridgeInstall.ShouldRefresh(
            "{\"statusLine\":{\"type\":\"command\",\"command\":\"\\\"C:/data/sobreiro-bridge.exe\\\"\"}}",
            "C:/data\\sobreiro-bridge.exe"));
}
