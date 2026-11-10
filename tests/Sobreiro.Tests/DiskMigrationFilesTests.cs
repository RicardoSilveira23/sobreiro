using Sobreiro.Core.Migration;

namespace Sobreiro.Tests;

public class DiskMigrationFilesTests : IDisposable
{
    private readonly string _root = Path.Combine(Path.GetTempPath(), "sobreiro-tests-" + Guid.NewGuid().ToString("N"));
    private readonly DiskMigrationFiles _files = new();

    public DiskMigrationFilesTests() => Directory.CreateDirectory(_root);

    public void Dispose() => Directory.Delete(_root, recursive: true);

    private string Make(string name)
    {
        var dir = Path.Combine(_root, name);
        Directory.CreateDirectory(Path.Combine(dir, "sub"));
        File.WriteAllText(Path.Combine(dir, "config.json"), "{}");
        File.WriteAllText(Path.Combine(dir, "sub", "log.txt"), "hello");
        return dir;
    }

    [Fact]
    public void Move_renames_the_folder_with_its_contents()
    {
        var from = Make("old");
        var to = Path.Combine(_root, "new");
        _files.MoveDirectory(from, to);

        Assert.False(Directory.Exists(from));
        Assert.Equal("hello", File.ReadAllText(Path.Combine(to, "sub", "log.txt")));
    }

    [Fact]
    public void Copy_skips_a_locked_file_and_counts_it()
    {
        var from = Make("old");
        var to = Path.Combine(_root, "new");
        using (new FileStream(Path.Combine(from, "sub", "log.txt"), FileMode.Open, FileAccess.Read, FileShare.None))
        {
            Assert.Equal(1, _files.CopyDirectory(from, to));
        }
        Assert.True(File.Exists(Path.Combine(to, "config.json")));
        Assert.False(File.Exists(Path.Combine(to, "sub", "log.txt")));
        Assert.True(File.Exists(Path.Combine(from, "sub", "log.txt")));
    }

    [Fact]
    public void Move_of_a_folder_holding_a_locked_file_throws_and_leaves_it_in_place()
    {
        var from = Make("old");
        var to = Path.Combine(_root, "new");
        using (new FileStream(Path.Combine(from, "sub", "log.txt"), FileMode.Open, FileAccess.Read, FileShare.None))
        {
            Assert.ThrowsAny<Exception>(() => _files.MoveDirectory(from, to));
        }
        Assert.True(Directory.Exists(from));
        Assert.False(Directory.Exists(to));
    }
}
