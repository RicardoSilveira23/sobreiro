using Sobreiro.Core;
using Sobreiro.Core.Usage;

namespace Sobreiro.Tests;

[CollectionDefinition("env", DisableParallelization = true)]
public class EnvCollection;

[Collection("env")]
public class UsageFileTests : IDisposable
{
    private readonly string _dir = Path.Combine(Path.GetTempPath(), "sobreiro-tests-" + Guid.NewGuid().ToString("N"));

    public void Dispose()
    {
        if (Directory.Exists(_dir)) Directory.Delete(_dir, recursive: true);
    }

    private static UsageSnapshot Sample() => new(
        new RateLimits(
            new LimitWindow(23.5, DateTimeOffset.FromUnixTimeSeconds(1738425600)),
            new LimitWindow(41.2, DateTimeOffset.FromUnixTimeSeconds(1738857600)),
            null),
        DateTimeOffset.FromUnixTimeSeconds(1738400000));

    [Fact]
    public void Write_then_Read_round_trips()
    {
        var path = Path.Combine(_dir, "usage.json");
        var snapshot = Sample();

        UsageFile.Write(path, snapshot);

        Assert.Equal(snapshot, UsageFile.Read(path));
    }

    [Fact]
    public void Write_creates_missing_directory()
    {
        var path = Path.Combine(_dir, "a", "b", "usage.json");

        UsageFile.Write(path, Sample());

        Assert.True(File.Exists(path));
    }

    [Fact]
    public void Write_replaces_existing_file_and_leaves_no_temp_files()
    {
        var path = Path.Combine(_dir, "usage.json");

        UsageFile.Write(path, Sample());
        UsageFile.Write(path, Sample());

        Assert.Equal([path], Directory.GetFiles(_dir));
    }

    [Fact]
    public void ReadForMerge_gives_the_stored_snapshot()
    {
        var path = Path.Combine(_dir, "usage.json");
        UsageFile.Write(path, Sample());

        Assert.True(UsageFile.TryReadForMerge(path, out var stored));
        Assert.Equal(Sample(), stored);
    }

    [Fact]
    public void ReadForMerge_of_a_missing_file_is_safe_to_write()
    {
        Assert.True(UsageFile.TryReadForMerge(Path.Combine(_dir, "usage.json"), out var stored));
        Assert.Null(stored);
    }

    [Fact]
    public void ReadForMerge_of_unparseable_content_is_safe_to_write_over()
    {
        Directory.CreateDirectory(_dir);
        var path = Path.Combine(_dir, "usage.json");
        File.WriteAllText(path, "not json");

        Assert.True(UsageFile.TryReadForMerge(path, out var stored));
        Assert.Null(stored);
    }

    // A writer holding the file must not make a merge drop what the file holds.
    [Fact]
    public void ReadForMerge_of_a_file_held_by_a_writer_is_not_safe()
    {
        var path = Path.Combine(_dir, "usage.json");
        UsageFile.Write(path, Sample());
        using var held = new FileStream(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None);

        Assert.False(UsageFile.TryReadForMerge(path, out var stored));
        Assert.Null(stored);
    }

    [Fact]
    public void Read_returns_null_for_missing_file()
    {
        Assert.Null(UsageFile.Read(Path.Combine(_dir, "usage.json")));
    }

    [Fact]
    public void Read_returns_null_for_corrupt_file()
    {
        Directory.CreateDirectory(_dir);
        var path = Path.Combine(_dir, "usage.json");
        File.WriteAllText(path, "{");

        Assert.Null(UsageFile.Read(path));
    }

    [Fact]
    public void Read_returns_null_while_file_is_locked()
    {
        var path = Path.Combine(_dir, "usage.json");
        UsageFile.Write(path, Sample());

        using var locked = new FileStream(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None);

        Assert.Null(UsageFile.Read(path));
    }

    [Fact]
    public void AppPaths_uses_SOBREIRO_DATA_DIR_when_set()
    {
        var old = Environment.GetEnvironmentVariable("SOBREIRO_DATA_DIR");
        try
        {
            Environment.SetEnvironmentVariable("SOBREIRO_DATA_DIR", _dir);

            Assert.Equal(_dir, AppPaths.DataDirectory());
            Assert.Equal(Path.Combine(_dir, "usage.json"), AppPaths.UsageFile());
            Assert.Equal(Path.Combine(_dir, "config.json"), AppPaths.ConfigFile());
        }
        finally
        {
            Environment.SetEnvironmentVariable("SOBREIRO_DATA_DIR", old);
        }
    }

    [Fact]
    public void AppPaths_ignores_relative_SOBREIRO_DATA_DIR()
    {
        var old = Environment.GetEnvironmentVariable("SOBREIRO_DATA_DIR");
        try
        {
            Environment.SetEnvironmentVariable("SOBREIRO_DATA_DIR", @"relative\dir");

            Assert.Equal(
                Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Sobreiro"),
                AppPaths.DataDirectory());
        }
        finally
        {
            Environment.SetEnvironmentVariable("SOBREIRO_DATA_DIR", old);
        }
    }

    [Fact]
    public void AppPaths_defaults_to_LocalApplicationData_when_unset()
    {
        var old = Environment.GetEnvironmentVariable("SOBREIRO_DATA_DIR");
        try
        {
            Environment.SetEnvironmentVariable("SOBREIRO_DATA_DIR", "");

            Assert.Equal(
                Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Sobreiro"),
                AppPaths.DataDirectory());
        }
        finally
        {
            Environment.SetEnvironmentVariable("SOBREIRO_DATA_DIR", old);
        }
    }
}
