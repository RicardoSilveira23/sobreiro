namespace Sobreiro.Core.Diagnostics;

/// <summary>User-facing text for diagnostics.</summary>
public static class DiagnosticsMessage
{
    public static string StartupFailed(string exceptionTypeName, string logPath) =>
        $"Sobreiro couldn't start ({exceptionTypeName}).\n\nThe details are in {logPath}";

    public static string StartupFailedUnlogged(string exceptionTypeName, string folder) =>
        $"Sobreiro couldn't start ({exceptionTypeName}).\n\nIt couldn't write its log either. Check that this folder can be created and written to: {folder}";
}
