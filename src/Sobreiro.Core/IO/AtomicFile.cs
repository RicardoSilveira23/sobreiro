using System.Text;

namespace Sobreiro.Core.IO;

/// <summary>Writes a file by writing a temp file beside it and renaming it over the target.</summary>
public static class AtomicFile
{
    private const int Attempts = 5;
    private static readonly TimeSpan RetryDelay = TimeSpan.FromMilliseconds(20);

    // A running bridge holds its exe for about 60 ms; 10 x 100 ms rides that out.
    private const int StreamAttempts = 10;
    private static readonly TimeSpan StreamRetryDelay = TimeSpan.FromMilliseconds(100);

    public static void WriteAllText(string path, string text, bool utf8Bom = false) =>
        Replace(path, temp => File.WriteAllText(temp, text, new UTF8Encoding(encoderShouldEmitUTF8Identifier: utf8Bom)),
            Attempts, RetryDelay);

    /// <summary>Writes <paramref name="content"/> to <paramref name="path"/>, retrying the rename for about a second.</summary>
    public static void Write(string path, Stream content) =>
        Replace(path, temp =>
        {
            using var file = new FileStream(temp, FileMode.CreateNew, FileAccess.Write, FileShare.None);
            content.CopyTo(file);
        }, StreamAttempts, StreamRetryDelay);

    private static void Replace(string path, Action<string> writeTemp, int attempts, TimeSpan delay)
    {
        var directory = Path.GetDirectoryName(Path.GetFullPath(path))!;
        Directory.CreateDirectory(directory);

        var temp = $"{path}.{Guid.NewGuid():N}.tmp";
        try
        {
            writeTemp(temp);
            for (var attempt = 1; ; attempt++)
            {
                try
                {
                    File.Move(temp, path, overwrite: true);
                    return;
                }
                catch (Exception e) when (attempt < attempts && e is IOException or UnauthorizedAccessException)
                {
                    Thread.Sleep(delay);
                }
            }
        }
        catch
        {
            try
            {
                File.Delete(temp);
            }
            catch (Exception e) when (e is IOException or UnauthorizedAccessException)
            {
            }
            throw;
        }
    }
}
