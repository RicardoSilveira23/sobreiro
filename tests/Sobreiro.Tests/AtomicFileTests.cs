using Sobreiro.Core.IO;

namespace Sobreiro.Tests;

public class AtomicFileTests : IDisposable
{
    private readonly string _dir = Path.Combine(Path.GetTempPath(), "sobreiro-tests-" + Guid.NewGuid().ToString("N"));

    public void Dispose()
    {
        if (Directory.Exists(_dir)) Directory.Delete(_dir, recursive: true);
    }

    [Fact]
    public void Writes_the_text_without_a_BOM_by_default()
    {
        var path = Path.Combine(_dir, "a.txt");
        AtomicFile.WriteAllText(path, "héllo");
        var bytes = File.ReadAllBytes(path);
        Assert.Equal("héllo", System.Text.Encoding.UTF8.GetString(bytes));
        Assert.NotEqual(0xEF, bytes[0]);
    }

    [Fact]
    public void Writes_a_BOM_when_asked()
    {
        var path = Path.Combine(_dir, "a.txt");
        AtomicFile.WriteAllText(path, "x", utf8Bom: true);
        Assert.Equal(new byte[] { 0xEF, 0xBB, 0xBF, (byte)'x' }, File.ReadAllBytes(path));
    }

    [Fact]
    public void Creates_a_missing_folder_and_replaces_an_existing_file()
    {
        var path = Path.Combine(_dir, "deep", "er", "a.txt");
        AtomicFile.WriteAllText(path, "one");
        AtomicFile.WriteAllText(path, "two");
        Assert.Equal("two", File.ReadAllText(path));
    }

    [Fact]
    public void A_failed_rename_leaves_no_temp_file_and_throws_the_original_error()
    {
        Directory.CreateDirectory(_dir);
        var path = Path.Combine(_dir, "a.txt");
        File.WriteAllText(path, "old");
        using var held = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.None);
        Assert.ThrowsAny<Exception>(() => AtomicFile.WriteAllText(path, "new"));
        Assert.Empty(Directory.GetFiles(_dir, "*.tmp"));
    }

    [Fact]
    public void Leaves_no_temp_file_behind()
    {
        AtomicFile.WriteAllText(Path.Combine(_dir, "a.txt"), "x");
        Assert.Empty(Directory.GetFiles(_dir, "*.tmp"));
    }

    [Fact]
    public void Write_from_a_stream_replaces_the_file()
    {
        Directory.CreateDirectory(_dir);
        var path = Path.Combine(_dir, "a.bin");
        File.WriteAllText(path, "old");
        AtomicFile.Write(path, new MemoryStream("new"u8.ToArray()));
        Assert.Equal("new", File.ReadAllText(path));
        Assert.Empty(Directory.GetFiles(_dir, "*.tmp"));
    }
}
