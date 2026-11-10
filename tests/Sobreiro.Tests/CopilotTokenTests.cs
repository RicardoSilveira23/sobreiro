using System.Text;
using Sobreiro.Core.Copilot;

namespace Sobreiro.Tests;

public class CopilotTokenTests
{
    private static readonly string Fake = "gho_" + new string('a', 36);

    [Fact] public void Finds_a_bare_token() => Assert.Equal(Fake, CopilotToken.Find(Fake));

    [Fact]
    public void Finds_a_token_inside_json() =>
        Assert.Equal(Fake, CopilotToken.Find($"{{\"oauth_token\":\"{Fake}\",\"user\":\"x\"}}"));

    [Fact]
    public void Finds_a_fine_grained_token()
    {
        var pat = "github_pat_11AB" + new string('c', 30) + "_" + new string('d', 20);
        Assert.Equal(pat, CopilotToken.Find($"token={pat} "));
    }

    [Fact]
    public void Text_without_a_token_gives_null()
    {
        Assert.Null(CopilotToken.Find("nothing to see here"));
        Assert.Null(CopilotToken.Find(null));
    }

    [Fact]
    public void Utf16_blob_with_bom_is_decoded()
    {
        var blob = Encoding.Unicode.GetPreamble().Concat(Encoding.Unicode.GetBytes($"{{\"oauth_token\":\"{Fake}\"}}")).ToArray();
        Assert.Equal(Fake, CopilotToken.FromBlob(blob));
    }

    [Fact]
    public void Utf8_blob_is_decoded()
    {
        var blob = Encoding.UTF8.GetPreamble().Concat(Encoding.UTF8.GetBytes($"{{\"oauth_token\":\"{Fake}\"}}")).ToArray();
        Assert.Equal(Fake, CopilotToken.FromBlob(blob));
    }

    [Fact]
    public void Utf8_blob_with_a_trailing_NUL_is_decoded()
    {
        var blob = Encoding.UTF8.GetBytes($"{{\"oauth_token\":\"{Fake}\"}}").Append((byte)0).ToArray();
        Assert.Equal(Fake, CopilotToken.FromBlob(blob));
    }

    [Fact]
    public void Environment_order_is_copilot_gh_github()
    {
        var env = new Dictionary<string, string> { ["GITHUB_TOKEN"] = "c", ["GH_TOKEN"] = "b", ["COPILOT_GITHUB_TOKEN"] = "a" };
        Assert.Equal(["a", "b", "c"], CopilotToken.FromEnvironment(k => env.GetValueOrDefault(k)));
    }

    [Fact]
    public void Blank_and_duplicate_environment_values_are_skipped()
    {
        var env = new Dictionary<string, string> { ["COPILOT_GITHUB_TOKEN"] = "  ", ["GH_TOKEN"] = " x ", ["GITHUB_TOKEN"] = "x" };
        Assert.Equal(["x"], CopilotToken.FromEnvironment(k => env.GetValueOrDefault(k)));
    }
}
