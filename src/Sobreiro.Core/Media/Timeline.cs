using Sobreiro.Core.Display;
using Sobreiro.Core.Limits;

namespace Sobreiro.Core.Media;

public enum AnimationKind { Hero, Styles, Day, CopilotJoins, Theme }

/// <summary>One still moment: which services, their readings, the look, and whether the details are open.</summary>
public sealed record Scene(Services Services, IReadOnlyList<Limit> Limits, FlyoutStyle Style, bool Dark, bool DetailsOpen);

/// <summary>Pointer offset from the pill's centre, in 1x pixels.</summary>
public readonly record struct PointerSpot(double Dx, double Dy);

public abstract record Beat(int Ms);

public sealed record Hold(Scene Scene, int Ms, PointerSpot? Pointer = null) : Beat(Ms);

public sealed record Glide(Scene Scene, PointerSpot From, PointerSpot To, int Ms) : Beat(Ms);

public sealed record Frame(Scene Scene, PointerSpot? Pointer, int Ms);

public static class Timeline
{
    private static int Round(double v) => (int)Math.Round(v, MidpointRounding.AwayFromZero);

    public static IReadOnlyList<Frame> Frames(IReadOnlyList<Beat> beats)
    {
        var frames = new List<Frame>();
        void Add(Frame f)
        {
            if (frames.Count > 0 && ReferenceEquals(frames[^1].Scene, f.Scene) && frames[^1].Pointer == f.Pointer)
                frames[^1] = frames[^1] with { Ms = frames[^1].Ms + f.Ms };
            else frames.Add(f);
        }
        foreach (var beat in beats)
        {
            switch (beat)
            {
                case Hold h:
                    Add(new Frame(h.Scene, h.Pointer, h.Ms));
                    break;
                case Glide g:
                    var n = Math.Max(1, Round(g.Ms * 60 / 1000.0));
                    for (var i = 0; i < n; i++)
                    {
                        var ms = Round((i + 1) * (double)g.Ms / n) - Round(i * (double)g.Ms / n);
                        var e = Ease((i + 1.0) / n);
                        var at = i == n - 1
                            ? g.To
                            : new PointerSpot(g.From.Dx + (g.To.Dx - g.From.Dx) * e, g.From.Dy + (g.To.Dy - g.From.Dy) * e);
                        Add(new Frame(g.Scene, at, ms));
                    }
                    break;
            }
        }
        return frames;
    }

    /// <summary>The index in <see cref="Frames"/> of the first frame of beat <paramref name="posterBeat"/>, after merging.</summary>
    public static int PosterIndex(IReadOnlyList<Beat> beats, int posterBeat)
    {
        var before = Frames(beats.Take(posterBeat).ToList()).Count;
        var through = Frames(beats.Take(posterBeat + 1).ToList()).Count;
        // A beat that merged into the previous frame adds none: its frame is the one before.
        return through > before ? before : Math.Max(before - 1, 0);
    }

    /// <summary>Ease-in-out cubic.</summary>
    public static double Ease(double t) => t < 0.5 ? 4 * t * t * t : 1 - Math.Pow(-2 * t + 2, 3) / 2;
}
