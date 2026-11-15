using Sobreiro.Core.Display;
using Sobreiro.Core.Usage;

namespace Sobreiro.Core.Bridge;

/// <summary>The bridge's whole flow. Always prints something and never throws.</summary>
public sealed class BridgeRunner(string usagePath, string configPath, Func<string, string, ChainResult?> runChain, bool insideChain = false)
{
    public int Run(string stdin, TextWriter stdout, DateTimeOffset now)
    {
        RateLimits? limits = null;
        try
        {
            limits = RateLimitsJson.ParseStatusLine(stdin);
            if (limits is not null) UsageFile.Write(usagePath, UsageMerge.Merge(UsageFile.Read(usagePath), limits, now));
        }
        catch (Exception)
        {
            // A failed write must not blank the user's status line.
        }

        try
        {
            if (!insideChain && BridgeConfig.Load(configPath).ChainedCommand is { } command && runChain(command, stdin) is { } chained)
            {
                stdout.Write(chained.Output);
                return chained.ExitCode;
            }
        }
        catch (Exception)
        {
            // Fall through to the default line.
        }

        stdout.Write(StatusLineText.Render(limits) + "\n");
        return 0;
    }
}
