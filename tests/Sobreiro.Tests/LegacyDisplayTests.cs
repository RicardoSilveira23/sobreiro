using System.Globalization;
using Sobreiro.Core;
using Sobreiro.Core.Config;
using Sobreiro.Core.Connect;
using Sobreiro.Core.Display;

namespace Sobreiro.Tests;

/// <summary>The words and paths that go with a legacy (OpenHeadroom) connection.</summary>
[Collection("env")]
public class LegacyDisplayTests
{
    private static readonly DateTimeOffset Now = DateTimeOffset.Parse("2026-11-09T10:00:00Z");
    private static readonly CultureInfo En = CultureInfo.GetCultureInfo("en-US");

    private const string OldBridgeBefore =
        "{ \"type\": \"command\", \"command\": \"\\\"C:/Apps/OpenHeadroom/openheadroom-bridge.exe\\\"\" }";

    private static ConnectionPlan Plan(bool reconnects) =>
        new(true, new SettingsSnapshot(@"C:\c\settings.json", true, "{}", false), reconnects ? OldBridgeBefore : "old", "new", "{}", AppConfig.Default, false, reconnects);

    [Fact]
    public void Reconnect_wording()
    {
        Assert.Equal("Reconnect to Claude Code", ConnectMessage.ReconnectTitle);
        Assert.Equal(
            "Reconnected. The numbers arrive with Claude Code's next response.",
            ConnectMessage.Done(true, null, reconnect: true));
        Assert.Contains("old OpenHeadroom bridge", ConnectMessage.Confirm(Plan(true)));
        Assert.DoesNotContain("OpenHeadroom", ConnectMessage.Confirm(Plan(false)));
    }

    [Fact]
    public void Tooltip_asks_to_reconnect_when_there_is_no_data_on_a_legacy_connection()
    {
        var text = TooltipText.Render([], Now, TimeZoneInfo.Utc, En, legacyConnection: true);

        Assert.Equal("Sobreiro · no data yet\nStill on the old bridge: reconnect from the tray menu.", text);
        Assert.True(text.Length <= TooltipText.MaxLength);
        Assert.Equal("Sobreiro · no data yet", TooltipText.Render([], Now, TimeZoneInfo.Utc, En));
    }

    [Fact]
    public void Flyout_footer_asks_to_reconnect_when_there_is_no_data_on_a_legacy_connection()
    {
        var model = FlyoutModel.From([], Now, TimeZoneInfo.Utc, En, legacyConnection: true);

        Assert.Equal(
            "No numbers yet. The status line still runs the old OpenHeadroom bridge: choose Reconnect in the tray menu.",
            model.Footer);
        Assert.Equal(
            "No numbers yet. They arrive with Claude Code's next response.",
            FlyoutModel.From([], Now, TimeZoneInfo.Utc, En).Footer);
    }

    private static T WithDataDir<T>(string? dir, Func<T> body)
    {
        var old = Environment.GetEnvironmentVariable("SOBREIRO_DATA_DIR");
        try
        {
            Environment.SetEnvironmentVariable("SOBREIRO_DATA_DIR", dir);
            return body();
        }
        finally
        {
            Environment.SetEnvironmentVariable("SOBREIRO_DATA_DIR", old);
        }
    }

    [Fact]
    public void Default_data_directory_is_recognised_with_or_without_an_override_naming_it()
    {
        var local = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        Assert.True(WithDataDir(null, AppPaths.IsDefaultDataDirectory));
        Assert.True(WithDataDir(Path.Combine(local, "SOBREIRO") + Path.DirectorySeparatorChar, AppPaths.IsDefaultDataDirectory));
        Assert.False(WithDataDir(Path.Combine(Path.GetTempPath(), "scratch"), AppPaths.IsDefaultDataDirectory));
    }

    [Fact]
    public void Default_and_legacy_directories_sit_side_by_side_in_local_app_data()
    {
        var local = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        Assert.Equal(Path.Combine(local, "Sobreiro"), AppPaths.DefaultDirectory());
        Assert.Equal(Path.Combine(local, "OpenHeadroom"), AppPaths.LegacyDirectory());
    }
}
