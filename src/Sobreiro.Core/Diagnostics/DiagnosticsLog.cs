using System.Text;

namespace Sobreiro.Core.Diagnostics;

/// <summary>
/// An append-only text log. When an entry would push the file past the cap, the file is moved to
/// log.1.txt (replacing it) and a new one starts. Nothing here ever throws.
/// </summary>
public sealed class DiagnosticsLog
{
    public const long DefaultMaxBytes = 262144;

    private static readonly UTF8Encoding Utf8 = new(encoderShouldEmitUTF8Identifier: false);
    private readonly object _gate = new();
    private readonly long _maxBytes;

    public DiagnosticsLog(string path, long maxBytes = DefaultMaxBytes)
    {
        FilePath = path;
        _maxBytes = maxBytes;
        PreviousFilePath = Path.Combine(
            Path.GetDirectoryName(path) ?? "",
            Path.GetFileNameWithoutExtension(path) + ".1" + Path.GetExtension(path));
    }

    public string FilePath { get; }

    public string PreviousFilePath { get; }

    /// <summary>Adds the entry as given. Returns false if it could not be written.</summary>
    public bool Append(string entry)
    {
        try
        {
            var bytes = Utf8.GetBytes(entry);
            lock (_gate)
            {
                var folder = Path.GetDirectoryName(FilePath);
                if (!string.IsNullOrEmpty(folder)) Directory.CreateDirectory(folder);
                Roll(bytes.Length);
                using var stream = new FileStream(
                    FilePath, FileMode.Append, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete);
                stream.Write(bytes);
            }
            return true;
        }
        catch (Exception)
        {
            return false;
        }
    }

    /// <summary>Formats an entry for now and this process, and appends it.</summary>
    public bool Write(string @event, object? exception = null) =>
        Append(LogEntry.Format(DateTimeOffset.Now, Environment.ProcessId, @event, exception));

    private void Roll(int incoming)
    {
        try
        {
            var info = new FileInfo(FilePath);
            if (info.Exists && info.Length > 0 && info.Length + incoming > _maxBytes)
                File.Move(FilePath, PreviousFilePath, overwrite: true);
        }
        catch (Exception)
        {
            // A failed roll must not stop the append.
        }
    }
}
