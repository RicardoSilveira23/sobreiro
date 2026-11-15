using Sobreiro.Core.Live;

namespace Sobreiro.Tests;

public class ClaudeCredentialsTests : IDisposable
{
    private readonly string _dir = Path.Combine(Path.GetTempPath(), "sobreiro-tests-" + Guid.NewGuid().ToString("N"));
    private static readonly DateTimeOffset Now = new(2026, 11, 9, 12, 0, 0, TimeSpan.Zero);
    private const string Secret = "sk-ant-oat01-SECRETVALUE";

    public void Dispose()
    {
        if (Directory.Exists(_dir)) Directory.Delete(_dir, recursive: true);
    }

    private string Write(string text)
    {
        Directory.CreateDirectory(_dir);
        var path = Path.Combine(_dir, ".credentials.json");
        File.WriteAllText(path, text);
        return path;
    }

    private static string Creds(string? expiresAt) =>
        "{\"claudeAiOauth\":{\"accessToken\":\"" + Secret + "\",\"refreshToken\":\"r\"" +
        (expiresAt is null ? "" : ",\"expiresAt\":" + expiresAt) + "}}";

    [Fact]
    public void A_future_expiry_gives_the_token()
    {
        var future = Now.AddHours(2).ToUnixTimeMilliseconds().ToString();
        var read = ClaudeCredentials.Read(Write(Creds(future)), Now);
        Assert.Equal(CredentialStatus.Ok, read.Status);
        Assert.Equal(Secret, read.Token!.Value);
        Assert.Equal(Now.AddHours(2), read.Token.ExpiresAt);
    }

    [Theory]
    [InlineData(-1)]
    [InlineData(0)]
    public void An_expiry_at_or_before_now_is_expired_and_returns_no_token(long offsetMs)
    {
        var at = (Now.ToUnixTimeMilliseconds() + offsetMs).ToString();
        var read = ClaudeCredentials.Read(Write(Creds(at)), Now);
        Assert.Equal(CredentialStatus.Expired, read.Status);
        Assert.Null(read.Token);
    }

    [Fact]
    public void No_expiry_is_ok_with_a_null_expiry()
    {
        var read = ClaudeCredentials.Read(Write(Creds(null)), Now);
        Assert.Equal(CredentialStatus.Ok, read.Status);
        Assert.Null(read.Token!.ExpiresAt);
    }

    [Fact]
    public void A_missing_file_is_missing()
    {
        var read = ClaudeCredentials.Read(Path.Combine(_dir, "nope.json"), Now);
        Assert.Equal(CredentialStatus.Missing, read.Status);
        Assert.Null(read.Token);
    }

    [Theory]
    [InlineData("{}")]
    [InlineData("[]")]
    [InlineData("{ broken")]
    [InlineData("")]
    [InlineData("{\"claudeAiOauth\":{\"accessToken\":12345}}")]
    [InlineData("{\"claudeAiOauth\":{\"accessToken\":\"\"}}")]
    [InlineData("{\"claudeAiOauth\":[]}")]
    public void Anything_without_a_token_string_is_unreadable(string text)
    {
        var read = ClaudeCredentials.Read(Write(text), Now);
        Assert.Equal(CredentialStatus.Unreadable, read.Status);
        Assert.Null(read.Token);
    }

    [Fact]
    public void A_locked_file_is_unreadable()
    {
        var path = Write(Creds(null));
        using var hold = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.None);
        Assert.Equal(CredentialStatus.Unreadable, ClaudeCredentials.Read(path, Now).Status);
    }

    // Claude Code may hold the file open for writing during its own refresh; reading must not need it closed.
    [Fact]
    public void A_file_held_open_for_writing_still_reads()
    {
        var path = Write(Creds(null));
        using var writer = new FileStream(path, FileMode.Open, FileAccess.Write, FileShare.Read);
        var read = ClaudeCredentials.Read(path, Now);
        Assert.Equal(CredentialStatus.Ok, read.Status);
        Assert.Equal(Secret, read.Token!.Value);
    }

    [Fact]
    public void A_byte_order_mark_is_skipped()
    {
        var path = Write("");
        File.WriteAllText(path, Creds(null), new System.Text.UTF8Encoding(encoderShouldEmitUTF8Identifier: true));
        Assert.Equal(CredentialStatus.Ok, ClaudeCredentials.Read(path, Now).Status);
    }

    [Fact]
    public void A_directory_in_the_way_is_unreadable_not_a_throw()
    {
        Directory.CreateDirectory(Path.Combine(_dir, "d"));
        Assert.Equal(CredentialStatus.Unreadable, ClaudeCredentials.Read(Path.Combine(_dir, "d"), Now).Status);
    }

    [Fact]
    public void Token_never_appears_in_ToString_or_results()
    {
        var read = ClaudeCredentials.Read(Write(Creds(null)), Now);
        Assert.Equal("AccessToken(redacted)", read.Token!.ToString());
        Assert.DoesNotContain(Secret, read.ToString());
        Assert.DoesNotContain(Secret, new LiveResult(LiveOutcome.Failed, null, null).ToString());
        Assert.DoesNotContain(Secret, $"{read.Token}");
    }

    [Theory]
    [InlineData("\"soon\"")]
    [InlineData("true")]
    [InlineData("{}")]
    public void A_present_expiry_that_is_not_a_number_is_unreadable(string expiry)
    {
        var read = ClaudeCredentials.Read(Write(Creds(expiry)), Now);
        Assert.Equal(CredentialStatus.Unreadable, read.Status);
        Assert.Null(read.Token);
    }

    [Fact]
    public void A_null_expiry_counts_as_none()
    {
        Assert.Equal(CredentialStatus.Ok, ClaudeCredentials.Read(Write(Creds("null")), Now).Status);
    }
}
