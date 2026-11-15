using Sobreiro.Core.Startup;

namespace Sobreiro.Core.Migration;

/// <summary>The file system, as far as the migration needs it.</summary>
public interface IMigrationFiles
{
    bool DirectoryExists(string path);

    /// <summary>Renames the folder. Throws when it can't (a locked file, say), leaving it where it was.</summary>
    void MoveDirectory(string from, string to);

    /// <summary>Copies what can be copied and skips the rest; returns how many files were skipped.</summary>
    int CopyDirectory(string from, string to);
}

/// <summary>The values under HKCU\...\Run.</summary>
public interface IRunValues
{
    string? Get(string name);

    void Set(string name, string command);

    void Delete(string name);
}

public enum DataFolderOutcome
{
    /// <summary>A data directory override is in use: nothing is touched.</summary>
    NotDefault,
    NothingToMove,
    BothExist,
    Moved,
    CopiedAfterMoveFailed,
}

public enum RunValueOutcome { Absent, Migrated, Failed }

public sealed record MigrationResult(bool OldAppRunning, DataFolderOutcome DataFolder, int FilesNotCopied, RunValueOutcome RunValue);

/// <param name="IsDefaultDataDirectory">False when a data directory override is in use.</param>
/// <param name="OldAppRunning">True while the old app's single-instance mutex is held.</param>
public sealed record MigrationContext(
    bool IsDefaultDataDirectory,
    string OldDataDirectory,
    string NewDataDirectory,
    string ExePath,
    Func<bool> OldAppRunning,
    IMigrationFiles Files,
    IRunValues RunValues);

/// <summary>
/// First run after the rename from OpenHeadroom: moves the data folder and the Start with Windows entry.
/// It never touches Claude Code's settings.json; the status line is repointed by Reconnect.
/// </summary>
public static class LegacyMigration
{
    public const string OldName = "OpenHeadroom";

    /// <summary>The old app's single-instance mutex for the default data folder.</summary>
    public const string OldMutexName = @"Local\OpenHeadroom.Tray";

    public const string StillRunningMessage =
        "OpenHeadroom is still running. Quit it from its tray menu, then start Sobreiro again.";

    public static MigrationResult Run(MigrationContext c)
    {
        if (!c.IsDefaultDataDirectory)
            return new MigrationResult(false, DataFolderOutcome.NotDefault, 0, RunValueOutcome.Absent);
        if (c.OldAppRunning())
            return new MigrationResult(true, DataFolderOutcome.NothingToMove, 0, RunValueOutcome.Absent);

        var (folder, missed) = MoveDataFolder(c);
        return new MigrationResult(false, folder, missed, MigrateRunValue(c));
    }

    private static (DataFolderOutcome, int) MoveDataFolder(MigrationContext c)
    {
        try
        {
            if (!c.Files.DirectoryExists(c.OldDataDirectory)) return (DataFolderOutcome.NothingToMove, 0);
            if (c.Files.DirectoryExists(c.NewDataDirectory)) return (DataFolderOutcome.BothExist, 0);
            try
            {
                c.Files.MoveDirectory(c.OldDataDirectory, c.NewDataDirectory);
                return (DataFolderOutcome.Moved, 0);
            }
            catch (Exception e) when (e is IOException or UnauthorizedAccessException)
            {
                // Carry on with a copy; the old folder stays where it is.
                return (DataFolderOutcome.CopiedAfterMoveFailed, c.Files.CopyDirectory(c.OldDataDirectory, c.NewDataDirectory));
            }
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            return (DataFolderOutcome.NothingToMove, 0);
        }
    }

    private static RunValueOutcome MigrateRunValue(MigrationContext c)
    {
        try
        {
            if (c.RunValues.Get(OldName) is null) return RunValueOutcome.Absent;
            c.RunValues.Set(AutoStartCommand.ValueName, AutoStartCommand.For(c.ExePath));
            c.RunValues.Delete(OldName);
            return RunValueOutcome.Migrated;
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException or System.Security.SecurityException)
        {
            return RunValueOutcome.Failed;
        }
    }
}

/// <summary>The real folder operations.</summary>
public sealed class DiskMigrationFiles : IMigrationFiles
{
    public bool DirectoryExists(string path) => Directory.Exists(path);

    public void MoveDirectory(string from, string to) => Directory.Move(from, to);

    public int CopyDirectory(string from, string to)
    {
        var missed = 0;
        try
        {
            Directory.CreateDirectory(to);
            foreach (var dir in Directory.EnumerateDirectories(from, "*", SearchOption.AllDirectories))
                Directory.CreateDirectory(Path.Combine(to, Path.GetRelativePath(from, dir)));
            foreach (var file in Directory.EnumerateFiles(from, "*", SearchOption.AllDirectories))
            {
                try
                {
                    File.Copy(file, Path.Combine(to, Path.GetRelativePath(from, file)), overwrite: false);
                }
                catch (Exception e) when (e is IOException or UnauthorizedAccessException)
                {
                    missed++;
                }
            }
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            missed++;
        }
        return missed;
    }
}
