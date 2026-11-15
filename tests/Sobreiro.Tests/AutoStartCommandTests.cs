using Sobreiro.Core.Startup;

namespace Sobreiro.Tests;

public class AutoStartCommandTests
{
    private const string Exe = @"C:\Apps\My Sobreiro\Sobreiro.exe";

    [Fact]
    public void Value_name_is_fixed() => Assert.Equal("Sobreiro", AutoStartCommand.ValueName);

    [Fact]
    public void For_quotes_the_path() => Assert.Equal("\"" + Exe + "\"", AutoStartCommand.For(Exe));

    [Theory]
    [InlineData("\"C:\\Apps\\My Sobreiro\\Sobreiro.exe\"")]
    [InlineData("\"c:\\apps\\my sobreiro\\sobreiro.EXE\"")]
    [InlineData("c:\\apps\\my sobreiro\\sobreiro.exe")]
    public void Matches_the_same_path_in_any_case_with_or_without_quotes(string value) =>
        Assert.True(AutoStartCommand.Matches(value, Exe));

    [Fact]
    public void Does_not_match_another_path_or_null()
    {
        Assert.False(AutoStartCommand.Matches("\"C:\\Other\\Sobreiro.exe\"", Exe));
        Assert.False(AutoStartCommand.Matches(null, Exe));
    }
}
