using Sobreiro.Core.IO;

namespace Sobreiro.Core.Connect;

/// <summary>Puts the bridge, which ships inside Sobreiro.exe, into the data folder where the status line runs it.</summary>
public static class BridgeInstall
{
    /// <summary>The name of the embedded bridge, and of the copy in the data folder.</summary>
    public const string ResourceName = "sobreiro-bridge.exe";

    /// <summary>
    /// Writes the bridge to <paramref name="path"/> unless it already holds the same bytes. Returns true when it wrote.
    /// Waits out a running bridge; throws <see cref="IOException"/> or <see cref="UnauthorizedAccessException"/> when it can't write.
    /// </summary>
    public static bool Ensure(Func<Stream> openSource, string path)
    {
        using (var source = openSource())
        {
            if (SameBytes(source, path)) return false;
        }
        using var content = openSource();
        AtomicFile.Write(path, content);
        return true;
    }

    /// <summary>True when the status line in <paramref name="settingsText"/> runs <paramref name="installedPath"/>.</summary>
    public static bool ShouldRefresh(string settingsText, string installedPath)
    {
        try
        {
            return BridgeCommand.Runs(SettingsJson.CommandOf(SettingsJson.ReadStatusLine(settingsText)), installedPath);
        }
        catch (FormatException)
        {
            return false;
        }
    }

    private static bool SameBytes(Stream source, string path)
    {
        if (!File.Exists(path)) return false;
        using var existing = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
        if (source.CanSeek && existing.Length != source.Length) return false;

        var a = new byte[8192];
        var b = new byte[8192];
        while (true)
        {
            var read = source.ReadAtLeast(a, a.Length, throwOnEndOfStream: false);
            var other = existing.ReadAtLeast(b, b.Length, throwOnEndOfStream: false);
            if (read != other || !a.AsSpan(0, read).SequenceEqual(b.AsSpan(0, other))) return false;
            if (read == 0) return true;
        }
    }
}
