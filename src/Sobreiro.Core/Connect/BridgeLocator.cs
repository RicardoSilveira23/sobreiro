namespace Sobreiro.Core.Connect;

public static class BridgeLocator
{
    private const string ExeName = "sobreiro-bridge.exe";

    /// <summary>
    /// This Sobreiro's bridge: the installed copy when the exe carries one, else the one found beside the app
    /// (<see cref="Find"/>); null when there is none.
    /// </summary>
    public static string? Own(bool carried, string installedPath, string appDirectory, Func<string, bool> fileExists) =>
        carried ? installedPath : Find(appDirectory, fileExists);

    /// <summary>The bridge beside the app, else in ..\bridge\; null when neither exists. Full paths.</summary>
    public static string? Find(string appDirectory, Func<string, bool> fileExists)
    {
        var beside = Path.GetFullPath(Path.Combine(appDirectory, ExeName));
        if (fileExists(beside)) return beside;

        var sibling = Path.GetFullPath(Path.Combine(appDirectory, "..", "bridge", ExeName));
        return fileExists(sibling) ? sibling : null;
    }
}
