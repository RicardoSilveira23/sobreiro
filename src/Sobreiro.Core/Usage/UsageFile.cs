using System.Text;
using Sobreiro.Core.IO;

namespace Sobreiro.Core.Usage;

public static class UsageFile
{
    private static readonly UTF8Encoding Utf8NoBom = new(encoderShouldEmitUTF8Identifier: false);

    public static void Write(string path, UsageSnapshot snapshot)
    {
        AtomicFile.WriteAllText(path, RateLimitsJson.WriteUsageFile(snapshot));
    }

    public static UsageSnapshot? Read(string path)
    {
        try
        {
            return RateLimitsJson.ParseUsageFile(ReadText(path));
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            return null;
        }
    }

    /// <summary>
    /// The snapshot to merge a new reading into. False when usage.json is there but can't be read now (a writer
    /// holds it): writing a merge then would drop what it holds. A missing file, or content that doesn't parse, is
    /// safe to write over and gives true with no snapshot.
    /// </summary>
    public static bool TryReadForMerge(string path, out UsageSnapshot? stored)
    {
        stored = null;
        string text;
        try
        {
            text = ReadText(path);
        }
        catch (Exception e) when (e is FileNotFoundException or DirectoryNotFoundException)
        {
            return !File.Exists(path);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            return false;
        }
        stored = RateLimitsJson.ParseUsageFile(text);
        return true;
    }

    private static string ReadText(string path)
    {
        using var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
        using var reader = new StreamReader(stream, Utf8NoBom);
        return reader.ReadToEnd();
    }
}
