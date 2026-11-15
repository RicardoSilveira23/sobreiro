using Microsoft.Win32;
using Sobreiro.Core.Startup;

namespace Sobreiro;

/// <summary>Start with Windows: the Sobreiro value under HKCU\...\Run, for this exe.</summary>
internal static class AutoStart
{
    private const string RunKey = @"Software\Microsoft\Windows\CurrentVersion\Run";

    /// <summary>True when the Run value starts this exe. A value for another copy counts as off.</summary>
    public static bool IsOn()
    {
        if (Environment.ProcessPath is not { } exe) return false;
        using var key = Registry.CurrentUser.OpenSubKey(RunKey);
        return AutoStartCommand.Matches(key?.GetValue(AutoStartCommand.ValueName) as string, exe);
    }

    /// <summary>On writes the value for this exe. Off removes it only when it starts this exe.</summary>
    public static void Set(bool on)
    {
        var exe = Environment.ProcessPath ?? throw new InvalidOperationException("Can't tell where Sobreiro.exe is.");
        using var key = Registry.CurrentUser.CreateSubKey(RunKey, writable: true);
        if (on)
            key.SetValue(AutoStartCommand.ValueName, AutoStartCommand.For(exe));
        else if (AutoStartCommand.Matches(key.GetValue(AutoStartCommand.ValueName) as string, exe))
            key.DeleteValue(AutoStartCommand.ValueName, throwOnMissingValue: false);
    }
}

/// <summary>The HKCU Run key, for the migration from OpenHeadroom.</summary>
internal sealed class RegistryRunValues : Sobreiro.Core.Migration.IRunValues
{
    private const string RunKey = @"Software\Microsoft\Windows\CurrentVersion\Run";

    public string? Get(string name)
    {
        using var key = Registry.CurrentUser.OpenSubKey(RunKey);
        return key?.GetValue(name) as string;
    }

    public void Set(string name, string command)
    {
        using var key = Registry.CurrentUser.CreateSubKey(RunKey, writable: true);
        key.SetValue(name, command);
    }

    public void Delete(string name)
    {
        using var key = Registry.CurrentUser.CreateSubKey(RunKey, writable: true);
        key.DeleteValue(name, throwOnMissingValue: false);
    }
}
