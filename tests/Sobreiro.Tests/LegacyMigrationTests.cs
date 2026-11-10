using Sobreiro.Core.Migration;
using Sobreiro.Core.Startup;

namespace Sobreiro.Tests;

public class LegacyMigrationTests
{
    private const string Old = @"C:\Users\x\AppData\Local\OpenHeadroom";
    private const string New = @"C:\Users\x\AppData\Local\Sobreiro";
    private const string Exe = @"C:\Apps\Sobreiro\Sobreiro.exe";

    private sealed class FakeFiles : IMigrationFiles
    {
        public HashSet<string> Directories { get; } = new(StringComparer.OrdinalIgnoreCase);
        public bool MoveFails { get; set; }
        public int CopyMisses { get; set; }
        public List<string> Log { get; } = [];

        public bool DirectoryExists(string path) => Directories.Contains(path);

        public void MoveDirectory(string from, string to)
        {
            Log.Add($"move {from} -> {to}");
            if (MoveFails) throw new IOException("a file is in use");
            Directories.Remove(from);
            Directories.Add(to);
        }

        public int CopyDirectory(string from, string to)
        {
            Log.Add($"copy {from} -> {to}");
            Directories.Add(to);
            return CopyMisses;
        }
    }

    private sealed class FakeRun : IRunValues
    {
        public Dictionary<string, string> Values { get; } = new();
        public bool Throws { get; set; }
        public string? Get(string name) => Values.GetValueOrDefault(name);

        public void Set(string name, string command)
        {
            if (Throws) throw new UnauthorizedAccessException();
            Values[name] = command;
        }

        public void Delete(string name) => Values.Remove(name);
    }

    private readonly FakeFiles _files = new();
    private readonly FakeRun _run = new();

    private MigrationResult Run(bool isDefault = true, bool oldRunning = false) =>
        LegacyMigration.Run(new MigrationContext(isDefault, Old, New, Exe, () => oldRunning, _files, _run));

    [Fact]
    public void Moves_the_old_folder_to_the_new_name()
    {
        _files.Directories.Add(Old);
        var result = Run();

        Assert.Equal(DataFolderOutcome.Moved, result.DataFolder);
        Assert.Equal([$"move {Old} -> {New}"], _files.Log);
        Assert.Contains(New, _files.Directories);
        Assert.DoesNotContain(Old, _files.Directories);
    }

    [Fact]
    public void Nothing_to_move_without_an_old_folder()
    {
        Assert.Equal(DataFolderOutcome.NothingToMove, Run().DataFolder);
        Assert.Empty(_files.Log);
    }

    [Fact]
    public void Leaves_both_folders_alone_when_both_exist()
    {
        _files.Directories.Add(Old);
        _files.Directories.Add(New);
        var result = Run();

        Assert.Equal(DataFolderOutcome.BothExist, result.DataFolder);
        Assert.Empty(_files.Log);
    }

    [Fact]
    public void Does_nothing_when_a_data_directory_override_is_in_use()
    {
        _files.Directories.Add(Old);
        _run.Values["OpenHeadroom"] = "\"C:\\old\\OpenHeadroom.exe\"";
        var result = Run(isDefault: false, oldRunning: true);

        Assert.Equal(DataFolderOutcome.NotDefault, result.DataFolder);
        Assert.False(result.OldAppRunning);
        Assert.Empty(_files.Log);
        Assert.Equal(["OpenHeadroom"], _run.Values.Keys);
    }

    [Fact]
    public void A_move_that_fails_copies_what_it_can_and_keeps_the_old_folder()
    {
        _files.Directories.Add(Old);
        _files.MoveFails = true;
        _files.CopyMisses = 2;
        var result = Run();

        Assert.Equal(DataFolderOutcome.CopiedAfterMoveFailed, result.DataFolder);
        Assert.Equal(2, result.FilesNotCopied);
        Assert.Contains(Old, _files.Directories);
        Assert.Equal($"copy {Old} -> {New}", _files.Log[^1]);
    }

    [Fact]
    public void Run_value_is_replaced_by_one_for_this_exe()
    {
        _run.Values["OpenHeadroom"] = "\"C:\\Apps\\OpenHeadroom\\OpenHeadroom.exe\"";
        var result = Run();

        Assert.Equal(RunValueOutcome.Migrated, result.RunValue);
        Assert.False(_run.Values.ContainsKey("OpenHeadroom"));
        Assert.Equal(AutoStartCommand.For(Exe), _run.Values["Sobreiro"]);
    }

    [Fact]
    public void Run_value_absent_changes_nothing()
    {
        var result = Run();

        Assert.Equal(RunValueOutcome.Absent, result.RunValue);
        Assert.Empty(_run.Values);
    }

    [Fact]
    public void A_registry_failure_leaves_the_old_value_and_does_not_throw()
    {
        _run.Values["OpenHeadroom"] = "x";
        _run.Throws = true;
        var result = Run();

        Assert.Equal(RunValueOutcome.Failed, result.RunValue);
        Assert.True(_run.Values.ContainsKey("OpenHeadroom"));
    }

    [Fact]
    public void Old_app_running_means_nothing_is_migrated()
    {
        _files.Directories.Add(Old);
        _run.Values["OpenHeadroom"] = "x";
        var result = Run(oldRunning: true);

        Assert.True(result.OldAppRunning);
        Assert.Empty(_files.Log);
        Assert.Equal(["OpenHeadroom"], _run.Values.Keys);
    }

    [Fact]
    public void Still_running_message_and_old_mutex_name_are_fixed()
    {
        Assert.Equal(
            "OpenHeadroom is still running. Quit it from its tray menu, then start Sobreiro again.",
            LegacyMigration.StillRunningMessage);
        Assert.Equal(@"Local\OpenHeadroom.Tray", LegacyMigration.OldMutexName);
    }
}
