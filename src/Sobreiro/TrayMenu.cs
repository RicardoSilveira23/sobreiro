using System.Diagnostics;
using System.IO;
using System.Windows.Forms;
using Sobreiro.Core;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Config;
using Sobreiro.Core.Connect;
using Sobreiro.Core.Display;
using Sobreiro.Core.Live;
using Application = System.Windows.Application;

namespace Sobreiro;

/// <summary>The right-click menu. Its checks and the Connect item are read fresh each time it opens.</summary>
internal sealed class TrayMenu : IDisposable
{
    private const string AboutText =
        "Shows Claude Code's session and weekly limits and GitHub Copilot's monthly AI credits. " + LiveUsageMessage.AboutLine + "\n\n"
        + "Copyright 2026 Ricardo Silveira. Free software under the GNU GPL v3 or later, with no warranty.\n\n"
        + "Not affiliated with or endorsed by Anthropic or GitHub.\n\n"
        + "Fonts: Inter and Unbounded, SIL Open Font License.";

    private readonly Action _redraw;
    private readonly ToolStripMenuItem _connect = new(ConnectMessage.Title(true));
    private readonly (ToolStripMenuItem Item, FlyoutStyle Style)[] _styles;
    private readonly (ToolStripMenuItem Item, BarColourOption Colour)[] _colours;
    private readonly (ToolStripMenuItem Item, ShowOption Show)[] _shows;
    private readonly (ToolStripMenuItem Item, PaceDays Days)[] _paces;
    private readonly ToolStripMenuItem _autoStart = new("Start with Windows");
    private readonly ToolStripMenuItem _disconnect = new(ConnectMessage.Title(false)) { Visible = false };
    private ConnectionState _state;
    private bool _busy;

    public TrayMenu(Action redraw)
    {
        _redraw = redraw;

        _styles =
        [
            (Item("Bars", () => SetStyle(FlyoutStyle.Bars)), FlyoutStyle.Bars),
            (Item("Battery", () => SetStyle(FlyoutStyle.Battery)), FlyoutStyle.Battery),
            (Item("Thermometer", () => SetStyle(FlyoutStyle.Thermometer)), FlyoutStyle.Thermometer),
        ];
        _colours =
        [
            (Item("Sobreiro purple", () => SetColour(BarColourOption.Purple)), BarColourOption.Purple),
            (Item("Windows accent", () => SetColour(BarColourOption.Accent)), BarColourOption.Accent),
        ];
        _shows =
        [
            (Item("Tray icon only", () => SetShow(ShowOption.TrayOnly)), ShowOption.TrayOnly),
            (Item("Tray icon and taskbar pill", () => SetShow(ShowOption.TrayAndPill)), ShowOption.TrayAndPill),
        ];
        _paces =
        [
            (Item("Every day", () => SetPace(PaceDays.EveryDay)), PaceDays.EveryDay),
            (Item("Weekdays only", () => SetPace(PaceDays.Weekdays)), PaceDays.Weekdays),
        ];
        _connect.Click += (_, _) => RunConnect(connect: _state != ConnectionState.Connected);
        _disconnect.Click += (_, _) => RunConnect(connect: false);
        _autoStart.Click += (_, _) => ToggleAutoStart();

        Strip.Items.AddRange(new ToolStripItem[]
        {
            _connect,
            _disconnect,
            new ToolStripMenuItem("Style", null, _styles.Select(s => (ToolStripItem)s.Item).ToArray()),
            new ToolStripMenuItem("Bar colour", null, _colours.Select(c => (ToolStripItem)c.Item).ToArray()),
            new ToolStripMenuItem("Pace", null, _paces.Select(p => (ToolStripItem)p.Item).ToArray()),
            new ToolStripMenuItem("Show", null, _shows.Select(s => (ToolStripItem)s.Item).ToArray()),
            new ToolStripSeparator(),
            _autoStart,
            Item("Open data folder", OpenDataFolder),
            Item("About Sobreiro", About),
            new ToolStripSeparator(),
            Item("Quit", () => Application.Current.Shutdown()),
        });
        Strip.Opening += OnOpening;
    }

    public ContextMenuStrip Strip { get; } = new();

    public static void ShowError(string text) =>
        MessageBox.Show(text, "Sobreiro", MessageBoxButtons.OK, MessageBoxIcon.Error);

    private static ToolStripMenuItem Item(string text, Action onClick) =>
        new(text, null, (_, _) => onClick());

    private void OnOpening(object? sender, System.ComponentModel.CancelEventArgs e)
    {
        _state = ConnectFlow.CurrentState();
        // A status line running the old OpenHeadroom bridge or another copy of ours: Reconnect, and Disconnect beside it.
        _connect.Text = _state switch
        {
            ConnectionState.Connected => ConnectMessage.Title(false),
            ConnectionState.Legacy or ConnectionState.Elsewhere => ConnectMessage.ReconnectTitle,
            _ => ConnectMessage.Title(true),
        };
        _disconnect.Visible = _state is ConnectionState.Legacy or ConnectionState.Elsewhere;

        var config = TrayController.ConfigForDisplay();
        foreach (var (item, style) in _styles) item.Checked = style == config.Style;
        foreach (var (item, colour) in _colours) item.Checked = colour == config.BarColour;
        foreach (var (item, days) in _paces) item.Checked = days == config.PaceDays;
        foreach (var (item, show) in _shows) item.Checked = show == config.Show;

        try
        {
            _autoStart.Checked = AutoStart.IsOn();
        }
        catch (Exception ex)
        {
            _autoStart.Checked = false;
            Debug.WriteLine($"Sobreiro: reading Start with Windows failed: {ex.GetType().Name}");
        }
    }

    // A message box pumps messages, so the menu can open again behind it: one flow at a time.
    private void RunConnect(bool connect)
    {
        if (_busy) return;
        _busy = true;
        try
        {
            ConnectFlow.Run(connect);
        }
        finally
        {
            _busy = false;
        }
    }

    private void SetStyle(FlyoutStyle style) => ChangeConfig(c => c with { Style = style });

    private void SetColour(BarColourOption colour) => ChangeConfig(c => c with { BarColour = colour });

    private void SetPace(PaceDays days) => ChangeConfig(c => c with { PaceDays = days });

    private void SetShow(ShowOption show) => ChangeConfig(c => c with { Show = show });

    /// <summary>
    /// Reload, change one field, save, redraw. A config that can't be read or parsed is never saved over.
    /// True when config.json now holds the change.
    /// </summary>
    private bool ChangeConfig(Func<AppConfig, AppConfig> change)
    {
        var path = AppPaths.ConfigFile();
        AppConfig config;
        try
        {
            config = AppConfig.Load(path);
        }
        catch (FormatException ex)
        {
            ShowError(ex.Message); // config.json isn't valid JSON: its own words say it was left alone
            return false;
        }
        catch (Exception ex)
        {
            ShowError("Couldn't read config.json, so nothing was saved.\n\n" + ex.Message);
            return false;
        }

        var next = change(config);
        if (next == config) return true;
        try
        {
            next.Save(path);
        }
        catch (Exception ex)
        {
            ShowError("Couldn't save config.json.\n\n" + ex.Message);
            return false;
        }
        _redraw();
        return true;
    }

    private static void ToggleAutoStart()
    {
        try
        {
            AutoStart.Set(!AutoStart.IsOn());
        }
        catch (Exception ex)
        {
            ShowError("Couldn't change Start with Windows.\n\n" + ex.Message);
        }
    }

    private static void OpenDataFolder()
    {
        try
        {
            var dir = AppPaths.DataDirectory();
            Directory.CreateDirectory(dir);
            Process.Start(new ProcessStartInfo("explorer.exe", $"\"{dir}\"") { UseShellExecute = true });
        }
        catch (Exception ex)
        {
            Debug.WriteLine($"Sobreiro: open data folder failed: {ex.GetType().Name}");
        }
    }

    private static void About()
    {
        var version = typeof(TrayMenu).Assembly.GetName().Version?.ToString(3) ?? "";
        var answer = MessageBox.Show(
            $"Sobreiro {version}\n\n{AboutText}\n\nRead the licences?",
            "About Sobreiro",
            MessageBoxButtons.YesNo, MessageBoxIcon.None, MessageBoxDefaultButton.Button2);
        if (answer == DialogResult.Yes) LicenceWindow.ShowLicences();
    }

    public void Dispose()
    {
        Strip.Opening -= OnOpening;
        Strip.Dispose(); // disposes the items, and their click handlers with them
    }
}
