using Sobreiro.Core.Bridge;

namespace Sobreiro.Tests;

public class BashLocatorTests
{
    private static string? Find(Dictionary<string, string> env, params string[] files)
    {
        var existing = new HashSet<string>(files, StringComparer.OrdinalIgnoreCase);
        return BashLocator.Find(k => env.GetValueOrDefault(k), existing.Contains);
    }

    [Fact]
    public void Prefers_CLAUDE_CODE_GIT_BASH_PATH_when_it_exists() =>
        Assert.Equal(@"D:\git\bash.exe", Find(
            new() { ["CLAUDE_CODE_GIT_BASH_PATH"] = @"D:\git\bash.exe", ["PATH"] = @"C:\Program Files\Git\usr\bin" },
            @"D:\git\bash.exe", @"C:\Program Files\Git\usr\bin\bash.exe"));

    [Fact]
    public void Finds_bash_on_PATH() =>
        Assert.Equal(@"C:\Program Files\Git\usr\bin\bash.exe", Find(
            new() { ["PATH"] = @"C:\Program Files\Git\usr\bin;C:\Windows\System32", ["SystemRoot"] = @"C:\Windows" },
            @"C:\Program Files\Git\usr\bin\bash.exe", @"C:\Windows\System32\bash.exe"));

    [Fact]
    public void Skips_System32_and_WindowsApps_bash() =>
        Assert.Null(Find(
            new() { ["PATH"] = @"C:\Windows\System32\;C:\Users\me\AppData\Local\Microsoft\WindowsApps", ["SystemRoot"] = @"C:\Windows" },
            @"C:\Windows\System32\bash.exe", @"C:\Users\me\AppData\Local\Microsoft\WindowsApps\bash.exe"));

    [Fact]
    public void Derives_bash_from_git_on_PATH() =>
        Assert.Equal(@"C:\Program Files\Git\bin\bash.exe", Find(
            new() { ["PATH"] = @"C:\Windows;;C:\Program Files\Git\cmd" },
            @"C:\Program Files\Git\cmd\git.exe", @"C:\Program Files\Git\bin\bash.exe"));

    [Fact]
    public void Falls_back_to_ProgramFiles_Git() =>
        Assert.Equal(@"C:\Program Files\Git\bin\bash.exe", Find(
            new() { ["PATH"] = @"C:\Windows", ["ProgramFiles"] = @"C:\Program Files" },
            @"C:\Program Files\Git\bin\bash.exe"));

    [Fact]
    public void Returns_null_when_nothing_found() =>
        Assert.Null(Find(new() { ["PATH"] = @"C:\Windows", ["ProgramFiles"] = @"C:\Program Files" }));
}
