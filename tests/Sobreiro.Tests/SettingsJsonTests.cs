using Sobreiro.Core.Connect;

namespace Sobreiro.Tests;

public class SettingsJsonTests
{
    private const string V = "{\"type\":\"command\"}";

    [Fact]
    public void Read_absent_is_null() => Assert.Null(SettingsJson.ReadStatusLine("{\"model\": \"x\"}"));

    [Fact]
    public void Read_blank_counts_as_empty_object()
    {
        Assert.Null(SettingsJson.ReadStatusLine(""));
        Assert.Null(SettingsJson.ReadStatusLine("  \n"));
    }

    [Fact]
    public void Read_returns_the_raw_value_text()
    {
        var text = "{\"statusLine\": {\"type\":\"command\",\"command\":\"x\"}}";
        Assert.Equal("{\"type\":\"command\",\"command\":\"x\"}", SettingsJson.ReadStatusLine(text));
    }

    // The messages are shown as they are in the Connect flow's error box.
    [Theory]
    [InlineData("[]", "settings.json isn't a JSON object, so Sobreiro left it alone.")]
    [InlineData("{oops", "settings.json isn't valid JSON, so Sobreiro left it alone.")]
    [InlineData("{} {}", "settings.json isn't valid JSON, so Sobreiro left it alone.")]
    public void Read_throws_FormatException_for_non_objects_and_bad_json(string text, string message) =>
        Assert.Equal(message, Assert.Throws<FormatException>(() => SettingsJson.ReadStatusLine(text)).Message);

    [Fact]
    public void Read_tolerates_comments_and_trailing_commas()
    {
        var text = "{\n  // a comment\n  \"statusLine\": \"x\", // more\n}";
        Assert.Equal("\"x\"", SettingsJson.ReadStatusLine(text));
    }

    [Fact]
    public void Replace_changes_only_the_value()
    {
        var text = "{\n    \"model\": \"café\",\n    // keep me\n    \"statusLine\": { \"type\": \"command\", \"command\": \"old\" },\n    \"theme\": \"dark\"\n}\n";
        var expected = "{\n    \"model\": \"café\",\n    // keep me\n    \"statusLine\": NEW,\n    \"theme\": \"dark\"\n}\n";
        Assert.Equal(expected, SettingsJson.WithStatusLine(text, "NEW"));
    }

    [Fact]
    public void Replace_keeps_comments_between_properties()
    {
        var text = "{\"statusLine\": 1, /* b */ \"a\": 2}";
        Assert.Equal("{\"statusLine\": 9, /* b */ \"a\": 2}", SettingsJson.WithStatusLine(text, "9"));
    }

    [Fact]
    public void Remove_first_property()
    {
        var text = "{\n  \"statusLine\": " + V + ",\n  \"a\": 1,\n  \"b\": 2\n}\n";
        Assert.Equal("{\n  \"a\": 1,\n  \"b\": 2\n}\n", SettingsJson.WithStatusLine(text, null));
    }

    [Fact]
    public void Remove_middle_property()
    {
        var text = "{\n  \"a\": 1,\n  \"statusLine\": " + V + ",\n  \"b\": 2\n}\n";
        Assert.Equal("{\n  \"a\": 1,\n  \"b\": 2\n}\n", SettingsJson.WithStatusLine(text, null));
    }

    [Fact]
    public void Remove_last_property_drops_the_previous_comma()
    {
        var text = "{\n  \"a\": 1,\n  \"b\": 2,\n  \"statusLine\": " + V + "\n}\n";
        Assert.Equal("{\n  \"a\": 1,\n  \"b\": 2\n}\n", SettingsJson.WithStatusLine(text, null));
    }

    [Fact]
    public void Remove_only_property_gives_empty_object_and_keeps_the_trailing_newline()
    {
        var text = "{\n  \"statusLine\": " + V + "\n}\n";
        Assert.Equal("{}\n", SettingsJson.WithStatusLine(text, null));
    }

    [Fact]
    public void Remove_on_one_line()
    {
        Assert.Equal("{\"a\":1}", SettingsJson.WithStatusLine("{\"a\":1,\"statusLine\":2}", null));
        Assert.Equal("{\"b\":1}", SettingsJson.WithStatusLine("{\"statusLine\":2, \"b\":1}", null));
        Assert.Equal("{\"a\":1, \"b\":1}", SettingsJson.WithStatusLine("{\"a\":1, \"statusLine\":2, \"b\":1}", null));
    }

    [Fact]
    public void Remove_when_absent_changes_nothing()
    {
        var text = "{\"a\": 1}\n";
        Assert.Equal(text, SettingsJson.WithStatusLine(text, null));
    }

    [Fact]
    public void Insert_appends_as_the_last_property()
    {
        Assert.Equal("{\n  \"model\": \"x\",\n  \"statusLine\": V\n}\n",
            SettingsJson.WithStatusLine("{\n  \"model\": \"x\"\n}\n", "V"));
    }

    [Fact]
    public void Insert_into_blank_text_creates_an_object()
    {
        Assert.Equal("{\n  \"statusLine\": V\n}\n", SettingsJson.WithStatusLine("", "V"));
    }

    [Fact]
    public void Insert_into_an_empty_object()
    {
        Assert.Equal("{\n  \"statusLine\": V\n}", SettingsJson.WithStatusLine("{}", "V"));
    }

    [Fact]
    public void Insert_keeps_crlf_and_indent()
    {
        Assert.Equal("{\r\n    \"a\": 1,\r\n    \"statusLine\": V\r\n}\r\n",
            SettingsJson.WithStatusLine("{\r\n    \"a\": 1\r\n}\r\n", "V"));
    }

    [Fact]
    public void CommandOf_reads_the_command_string()
    {
        Assert.Equal("x", SettingsJson.CommandOf("{\"type\":\"command\",\"command\":\"x\"}"));
        Assert.Null(SettingsJson.CommandOf(null));
        Assert.Null(SettingsJson.CommandOf("\"x\""));
        Assert.Null(SettingsJson.CommandOf("{\"type\":\"command\"}"));
    }
}
