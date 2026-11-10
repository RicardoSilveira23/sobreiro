using System.IO;
using Sobreiro.Core;
using Sobreiro.Core.Migration;

namespace Sobreiro;

/// <summary>Runs the first-run migration from OpenHeadroom against the real disk and registry.</summary>
internal static class LegacyMigrationHost
{
    public static MigrationResult Run() =>
        LegacyMigration.Run(new MigrationContext(
            AppPaths.IsDefaultDataDirectory(),
            AppPaths.LegacyDirectory(),
            AppPaths.DefaultDirectory(),
            Environment.ProcessPath ?? Path.Combine(AppContext.BaseDirectory, "Sobreiro.exe"),
            OldAppRunning,
            new DiskMigrationFiles(),
            new RegistryRunValues()));

    /// <summary>True while the old app holds its single-instance mutex.</summary>
    private static bool OldAppRunning()
    {
        try
        {
            using var mutex = Mutex.OpenExisting(LegacyMigration.OldMutexName);
            return true;
        }
        catch (WaitHandleCannotBeOpenedException)
        {
            return false;
        }
        catch (UnauthorizedAccessException)
        {
            return true; // it exists, but another session or user owns it
        }
    }
}
