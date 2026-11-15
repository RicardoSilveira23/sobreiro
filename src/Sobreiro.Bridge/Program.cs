using System.Text;
using Sobreiro.Core;
using Sobreiro.Core.Bridge;
using Sobreiro.Core.Display;

var utf8 = new UTF8Encoding(false);
using var stdout = new StreamWriter(Console.OpenStandardOutput(), utf8);
try
{
    string stdin;
    using (var reader = new StreamReader(Console.OpenStandardInput(), utf8))
        stdin = reader.ReadToEnd();

    var runner = new BridgeRunner(
        AppPaths.UsageFile(),
        AppPaths.ConfigFile(),
        (command, input) => ChainRunner.Run(command, input, BashLocator.Find(Environment.GetEnvironmentVariable, File.Exists)),
        Environment.GetEnvironmentVariable("SOBREIRO_IN_CHAIN") == "1");
    return runner.Run(stdin, stdout, DateTimeOffset.UtcNow);
}
catch (Exception)
{
    stdout.Write(StatusLineText.Render(null) + "\n");
    return 0;
}
