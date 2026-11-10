using System.Text;
using Sobreiro.Core.Diagnostics;

namespace Sobreiro.Tests;

public class DiagnosticsLogTests : IDisposable
{
    private readonly string _dir = Path.Combine(Path.GetTempPath(), "sobreiro-tests-" + Guid.NewGuid().ToString("N"));

    public void Dispose()
    {
        if (Directory.Exists(_dir)) Directory.Delete(_dir, recursive: true);
    }

    private string LogPath => Path.Combine(_dir, "log.txt");

    private string PreviousPath => Path.Combine(_dir, "log.1.txt");

    [Fact]
    public void Append_creates_the_folder_and_writes_utf8_without_bom()
    {
        var log = new DiagnosticsLog(Path.Combine(_dir, "sub", "log.txt"));
        Assert.True(log.Append("héllo\n"));
        var bytes = File.ReadAllBytes(log.FilePath);
        Assert.Equal("héllo\n", Encoding.UTF8.GetString(bytes));
        Assert.NotEqual(0xEF, bytes[0]);
    }

    [Fact]
    public void Append_adds_to_the_end()
    {
        var log = new DiagnosticsLog(LogPath);
        Assert.True(log.Append("one\n"));
        Assert.True(log.Append("two\n"));
        Assert.Equal("one\ntwo\n", File.ReadAllText(LogPath));
    }

    [Fact]
    public void Append_rolls_to_log_1_when_the_cap_would_be_passed()
    {
        var log = new DiagnosticsLog(LogPath, 100);
        var first = new string('a', 59) + "\n";
        var second = new string('b', 59) + "\n";
        Assert.True(log.Append(first));
        Assert.True(log.Append(second));
        Assert.Equal(second, File.ReadAllText(LogPath));
        Assert.Equal(first, File.ReadAllText(PreviousPath));
    }

    [Fact]
    public void Append_replaces_an_older_log_1()
    {
        var log = new DiagnosticsLog(LogPath, 100);
        Directory.CreateDirectory(_dir);
        File.WriteAllText(PreviousPath, "old\n");
        var first = new string('a', 59) + "\n";
        Assert.True(log.Append(first));
        Assert.True(log.Append(new string('b', 59) + "\n"));
        Assert.Equal(first, File.ReadAllText(PreviousPath));
    }

    [Fact]
    public void Append_never_rolls_an_empty_file()
    {
        var log = new DiagnosticsLog(LogPath, 100);
        var big = new string('a', 300) + "\n";
        Assert.True(log.Append(big));
        Assert.Equal(big, File.ReadAllText(LogPath));
        Assert.False(File.Exists(PreviousPath));
    }

    [Fact]
    public void Append_returns_false_when_the_path_is_a_directory()
    {
        var path = Path.Combine(_dir, "log.txt");
        Directory.CreateDirectory(path);
        Assert.False(new DiagnosticsLog(path).Append("x\n"));
    }

    [Fact]
    public void Previous_file_path_sits_beside_the_log() =>
        Assert.Equal(Path.Combine(_dir, "log.1.txt"), new DiagnosticsLog(LogPath).PreviousFilePath);

    [Fact]
    public void Write_uses_this_process_id()
    {
        var log = new DiagnosticsLog(LogPath);
        Assert.True(log.Write("hello event"));
        Assert.Contains($"pid={Environment.ProcessId} hello event", File.ReadAllText(LogPath));
    }
}
