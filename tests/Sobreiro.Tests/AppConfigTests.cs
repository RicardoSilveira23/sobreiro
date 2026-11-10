using Sobreiro.Core.Bridge;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Config;
using Sobreiro.Core.Display;

namespace Sobreiro.Tests;

public class AppConfigTests : IDisposable
{
    private readonly string _dir = Path.Combine(Path.GetTempPath(), "sobreiro-tests-" + Guid.NewGuid().ToString("N"));
    private string PathFor() => Path.Combine(_dir, "config.json");

    public void Dispose()
    {
        if (Directory.Exists(_dir)) Directory.Delete(_dir, recursive: true);
    }

    private string Write(string text)
    {
        Directory.CreateDirectory(_dir);
        File.WriteAllText(PathFor(), text);
        return PathFor();
    }

    private static void AssertDefault(AppConfig c)
    {
        Assert.Null(c.ChainedCommand);
        Assert.Null(c.Previous);
        Assert.Equal(FlyoutStyle.Bars, c.Style);
        Assert.Equal(BarColourOption.Purple, c.BarColour);
        Assert.Equal(ShowOption.TrayAndPill, c.Show);
        Assert.Empty(c.Extra);
    }

    [Fact] public void Show_defaults_to_tray_and_pill() => Assert.Equal(ShowOption.TrayAndPill, AppConfig.Load(PathFor()).Show);

    [Fact]
    public void Show_loads_and_unknown_values_fall_back()
    {
        var loaded = AppConfig.Load(Write("{\"show\":\"TrayOnly\"}"));
        Assert.Equal(ShowOption.TrayOnly, loaded.Show);
        Assert.DoesNotContain("show", loaded.Extra.Keys);
        Assert.Equal(ShowOption.TrayAndPill, AppConfig.Load(Write("{\"show\":\"everywhere\"}")).Show);
    }

    [Fact]
    public void Show_round_trips_and_is_written_lower_case()
    {
        (AppConfig.Default with { Show = ShowOption.TrayOnly }).Save(PathFor());
        Assert.Contains("\"show\": \"trayonly\"", File.ReadAllText(PathFor()));
        Assert.Equal(ShowOption.TrayOnly, AppConfig.Load(PathFor()).Show);
    }

    [Fact]
    public void Missing_file_gives_default() => AssertDefault(AppConfig.Load(PathFor()));

    [Theory]
    [InlineData("")]
    [InlineData("  \n\t ")]
    public void Empty_or_whitespace_gives_default(string text) => AssertDefault(AppConfig.Load(Write(text)));

    [Theory]
    [InlineData("{not json")]
    [InlineData("[1,2]")]
    [InlineData("\"text\"")]
    [InlineData("null")]
    public void Corrupt_or_non_object_throws_instead_of_giving_default(string text)
    {
        var e = Assert.Throws<FormatException>(() => AppConfig.Load(Write(text)));
        Assert.Equal("config.json isn't valid JSON, so Sobreiro left it alone.", e.Message);
        Assert.Equal(text, File.ReadAllText(PathFor()));
    }

    [Theory]
    [InlineData("")]
    [InlineData("   ")]
    public void A_blank_chained_command_is_no_command(string command) =>
        Assert.Null(AppConfig.Load(Write("{\"chained_command\":\"" + command + "\"}")).ChainedCommand);

    [Fact]
    public void Round_trips_every_field()
    {
        const string raw = "{\"type\":\"command\",\"command\":\"x\",\"padding\":0}";
        var config = new AppConfig("node a.js", new PreviousStatusLine(raw), FlyoutStyle.Thermometer, BarColourOption.Accent,
            new Dictionary<string, string> { ["future"] = "[1, 2]" });
        config.Save(PathFor());

        var loaded = AppConfig.Load(PathFor());
        Assert.Equal("node a.js", loaded.ChainedCommand);
        Assert.Equal(raw, loaded.Previous!.RawJson);
        Assert.Equal(FlyoutStyle.Thermometer, loaded.Style);
        Assert.Equal(BarColourOption.Accent, loaded.BarColour);
        Assert.Equal("[1, 2]", loaded.Extra["future"]);
    }

    [Theory]
    [InlineData("true")]
    [InlineData("false")]
    public void An_old_live_usage_key_loads_and_is_kept_untouched_as_an_unknown_key(string old)
    {
        Write("{\"live_usage\":" + old + ",\"future\":[1]}");
        var loaded = AppConfig.Load(PathFor());
        Assert.Equal(old, loaded.Extra["live_usage"]);
        Assert.Equal("[1]", loaded.Extra["future"]);
        loaded.Save(PathFor());
        var again = AppConfig.Load(PathFor());
        Assert.Equal(old, again.Extra["live_usage"]);
        Assert.Equal("[1]", again.Extra["future"]);
    }

    [Fact]
    public void Previous_status_line_object_text_comes_back_byte_for_byte()
    {
        const string raw = "{\"type\":\"command\",\"command\":\"x\",\"padding\":0}";
        Write("{\"previous_status_line\":" + raw + "}");
        var loaded = AppConfig.Load(PathFor());
        Assert.Equal(raw, loaded.Previous!.RawJson);
        loaded.Save(PathFor());
        Assert.Equal(raw, AppConfig.Load(PathFor()).Previous!.RawJson);
    }

    [Fact]
    public void Previous_null_is_distinct_from_absent()
    {
        Write("{\"previous_status_line\":null}");
        var present = AppConfig.Load(PathFor());
        Assert.NotNull(present.Previous);
        Assert.Null(present.Previous!.RawJson);

        present.Save(PathFor());
        var again = AppConfig.Load(PathFor());
        Assert.NotNull(again.Previous);
        Assert.Null(again.Previous!.RawJson);

        Write("{}");
        Assert.Null(AppConfig.Load(PathFor()).Previous);
    }

    [Fact]
    public void Unknown_key_survives_load_and_save()
    {
        Write("{\"future\": [1, 2], \"style\":\"battery\"}");
        AppConfig.Load(PathFor()).Save(PathFor());
        var again = AppConfig.Load(PathFor());
        Assert.Equal("[1, 2]", again.Extra["future"]);
        Assert.Equal(FlyoutStyle.Battery, again.Style);
    }

    [Fact]
    public void Style_and_colour_load_and_unknown_values_fall_back()
    {
        var c = AppConfig.Load(Write("{\"style\":\"battery\",\"bar_colour\":\"accent\"}"));
        Assert.Equal(FlyoutStyle.Battery, c.Style);
        Assert.Equal(BarColourOption.Accent, c.BarColour);

        var d = AppConfig.Load(Write("{\"style\":\"hologram\",\"bar_colour\":\"mauve\"}"));
        Assert.Equal(FlyoutStyle.Bars, d.Style);
        Assert.Equal(BarColourOption.Purple, d.BarColour);
    }

    [Fact]
    public void Saved_chained_command_is_read_by_BridgeConfig()
    {
        (AppConfig.Default with { ChainedCommand = "echo hi" }).Save(PathFor());
        Assert.Equal("echo hi", BridgeConfig.Load(PathFor()).ChainedCommand);
    }

    [Fact]
    public void A_file_that_cannot_be_read_throws_instead_of_giving_default()
    {
        Write("{\"chained_command\":\"x\"}");
        using var held = new FileStream(PathFor(), FileMode.Open, FileAccess.ReadWrite, FileShare.None);
        Assert.Throws<IOException>(() => AppConfig.Load(PathFor()));
    }

    [Fact]
    public void A_missing_folder_gives_default() =>
        AssertDefault(AppConfig.Load(Path.Combine(_dir, "nope", "config.json")));

    [Theory]
    [InlineData("1")]
    [InlineData("0")]
    public void Numeric_strings_are_not_enum_values(string number)
    {
        var c = AppConfig.Load(Write("{\"style\":\"" + number + "\",\"bar_colour\":\"" + number + "\"}"));
        Assert.Equal(FlyoutStyle.Bars, c.Style);
        Assert.Equal(BarColourOption.Purple, c.BarColour);
    }

    [Fact]
    public void Commands_are_written_readably()
    {
        const string command = "a && b \"c\"";
        (AppConfig.Default with { ChainedCommand = command }).Save(PathFor());
        Assert.Contains("\"chained_command\": \"a && b \\\"c\\\"\"", File.ReadAllText(PathFor()));
        Assert.Equal(command, BridgeConfig.Load(PathFor()).ChainedCommand);
        Assert.Equal(command, AppConfig.Load(PathFor()).ChainedCommand);
    }
}
