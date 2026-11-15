namespace Sobreiro.Core.Taskbar;

/// <summary>Decides whether a pill tick also asks UI Automation where the taskbar's parts are.</summary>
public readonly record struct TaskbarQuerySchedule
{
    /// <summary>
    /// How long a settled schedule goes without asking: layout changes that raise no foreground switch are
    /// noticed within about 16 seconds (15, landing on a 2-second tick).
    /// </summary>
    public static readonly TimeSpan SlowCheck = TimeSpan.FromSeconds(15);

    /// <summary>Questions a foreground switch owes: the first can land before the new window's button or tray icon exists.</summary>
    public const int AsksPerSwitch = 2;

    /// <summary>Questions (one minute of 2-second ticks) after which missing answers may settle, when no anchors were ever seen.</summary>
    public const int AsksBeforeAnchors = 30;

    // default(TaskbarQuerySchedule) is the fresh schedule: nothing asked, nothing answered.
    private bool HasAsked { get; init; }
    private TimeSpan LastAsked { get; init; }
    private int SwitchesOwed { get; init; }
    private bool Unsettled { get; init; }
    private bool HasAnswer { get; init; }
    private TaskbarAnchors? LastAnswer { get; init; }
    private bool AnchorsSeen { get; init; }
    private int AskCount { get; init; }

    /// <summary>
    /// True when nothing has been asked yet, a foreground switch still owes questions (two each), the last
    /// answers have not agreed yet, or <see cref="SlowCheck"/> has passed since the last question. A question
    /// whose answer never comes is therefore asked again after the slow check. A clock that went back is not due.
    /// </summary>
    public bool Due(TimeSpan now) =>
        !HasAsked || SwitchesOwed > 0 || Unsettled || now - LastAsked >= SlowCheck;

    /// <summary>A question went out at <paramref name="now"/>: it pays one owed switch question and is the one that settles the answers.</summary>
    public TaskbarQuerySchedule Asked(TimeSpan now) =>
        this with
        {
            HasAsked = true,
            LastAsked = now,
            SwitchesOwed = Math.Max(SwitchesOwed - 1, 0),
            Unsettled = false,
            AskCount = Math.Min(AskCount + 1, AsksBeforeAnchors),
        };

    /// <summary>
    /// The answer to a question; null when the taskbar could not be read, which is an answer like any other.
    /// The first answer, or one that differs from the answer before it, calls for another question. At the
    /// start, while no anchors have ever come and fewer than <see cref="AsksBeforeAnchors"/> questions were
    /// asked (the tray may not be built yet), a missing answer never settles it.
    /// </summary>
    public TaskbarQuerySchedule Answered(TaskbarAnchors? answer)
    {
        var anchorsSeen = AnchorsSeen || answer is not null;
        var waitingForTray = !anchorsSeen && AskCount < AsksBeforeAnchors;
        return this with
        {
            Unsettled = waitingForTray || !HasAnswer || answer != LastAnswer,
            HasAnswer = true,
            LastAnswer = answer,
            AnchorsSeen = anchorsSeen,
        };
    }

    /// <summary>The foreground window changed: the next two ticks ask.</summary>
    public TaskbarQuerySchedule ForegroundSwitched() => this with { SwitchesOwed = AsksPerSwitch };
}
