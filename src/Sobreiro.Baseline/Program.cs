using System.Diagnostics;
using System.Text;
using Sobreiro.Baseline;
using Sobreiro.Core.Baseline;

const string Usage = """
    sobreiro-baseline [--exe <Sobreiro.exe>] [--bridge <sobreiro-bridge.exe>] [--label <build>]
                      [--tray run|size] [--idle-minutes 10] [--starts 5] [--bridge-runs 20]
                      [--table <baseline.md> --section <heading>]

    Starts its own copy of the tray on a scratch data folder and measures it: time to the first
    pill, memory and processor time when idle, the size of both exes and the bridge's run time.
    With --tray size, or with no --exe, the tray is not started: only sizes and the bridge's run
    time are taken. Prints one table row; with --table, writes it into that file too.
    """;

var options = new Dictionary<string, string>();
for (var i = 0; i < args.Length; i += 2)
{
    if (!args[i].StartsWith("--") || i + 1 >= args.Length)
    {
        Console.Error.WriteLine(Usage);
        return 2;
    }
    options[args[i][2..]] = args[i + 1];
}
var exe = options.TryGetValue("exe", out var e) ? Path.GetFullPath(e) : null;
var bridgeExe = options.TryGetValue("bridge", out var b) ? Path.GetFullPath(b) : null;
var trayMode = options.GetValueOrDefault("tray", "run");
if ((exe ?? bridgeExe) is not { } labelled || trayMode is not ("run" or "size")
    || options.ContainsKey("table") != options.ContainsKey("section"))
{
    Console.Error.WriteLine(Usage);
    return 2;
}
var runTray = exe is not null && trayMode == "run";
var idleMinutes = Number("idle-minutes", 10, 2);
var starts = Number("starts", 5, 2);
var bridgeRuns = Number("bridge-runs", 20, 1);
var label = options.GetValueOrDefault("label") ?? VersionOf(labelled);

var scratch = Path.Combine(Path.GetTempPath(), "sobreiro-baseline-" + Guid.NewGuid().ToString("N")[..8]);
Directory.CreateDirectory(scratch);
void Say(string line) => Console.WriteLine($"{DateTime.Now:HH:mm:ss} {line}");

try
{
    if (runTray && Native.PillOfAnotherApp() is { } other)
    {
        Console.Error.WriteLine(
            $"A pill is already in the taskbar (process {other}, {NameOf(other)}). A second one finds no room beside it and " +
            "stays hidden, so quit that app first and start it again afterwards.");
        return 1;
    }
    using var timer = Native.FineTimer();
    var measure = new Measure(scratch, Say);
    Say($"build {label}: {labelled}");
    List<TimeSpan?> pills = [null];
    IdleFigures? idle = null;
    if (runTray)
    {
        Say($"{starts} starts, signed out");
        pills = measure.Starts(exe!, starts);
        Say($"idle for {idleMinutes} minutes, with this PC's sign-ins");
        idle = measure.Idle(exe!, idleMinutes);
    }
    TimeSpan? bridgeRun = null;
    if (bridgeExe is not null)
    {
        Say($"{bridgeRuns} bridge runs");
        bridgeRun = measure.Bridge(bridgeExe, bridgeRuns);
    }

    var run = new BaselineRun(
        label, DateOnly.FromDateTime(DateTime.Now), idle,
        pills[0], BaselineMath.Median(pills.Skip(1).OfType<TimeSpan>().ToList()),
        exe is null ? null : new FileInfo(exe).Length, bridgeExe is null ? null : new FileInfo(bridgeExe).Length, bridgeRun);
    Console.WriteLine(BaselineTable.Header);
    Console.WriteLine(BaselineTable.Divider);
    Console.WriteLine(BaselineTable.Row(run));

    if (options.TryGetValue("table", out var table))
    {
        var section = options["section"];
        if (BaselineTable.Add(File.ReadAllText(table), section, run) is not { } updated)
        {
            Console.Error.WriteLine($"No table under \"## {section}\" in {table}: nothing written.");
            return 1;
        }
        File.WriteAllText(table, updated, new UTF8Encoding(false));
        Say($"row written to {table}, under {section}");
    }
    return 0;
}
catch (Exception ex)
{
    Console.Error.WriteLine($"Baseline failed: {ex.Message}");
    return 1;
}
finally
{
    try
    {
        Directory.Delete(scratch, recursive: true);
    }
    catch (Exception)
    {
        Console.Error.WriteLine($"Could not remove {scratch}.");
    }
}

int Number(string name, int fallback, int least) =>
    options.TryGetValue(name, out var text) && int.TryParse(text, out var n) ? Math.Max(n, least) : fallback;

static string NameOf(int processId)
{
    try
    {
        using var process = Process.GetProcessById(processId);
        return process.ProcessName;
    }
    catch (Exception)
    {
        return "gone";
    }
}

// "0.10.0" from "0.10.0+<commit>".
static string VersionOf(string path) =>
    FileVersionInfo.GetVersionInfo(path).ProductVersion?.Split('+')[0] is { Length: > 0 } version ? version : "unknown";
