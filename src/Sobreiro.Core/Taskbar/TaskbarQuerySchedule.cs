namespace Sobreiro.Core.Taskbar;

/// <summary>Decides whether a pill tick also asks UI Automation where the taskbar's parts are.</summary>
public readonly record struct TaskbarQuerySchedule
{
    /// <summary>How long a settled schedule goes without asking: layout changes that raise no foreground switch are noticed within this.</summary>
    public static readonly TimeSpan SlowCheck = TimeSpan.FromSeconds(15);

    // default(TaskbarQuerySchedule) is the fresh schedule: nothing asked, nothing answered.
    private bool HasAsked { get; init; }
    private TimeSpan LastAsked { get; init; }
    private bool SwitchPending { get; init; }
    private bool Unsettled { get; init; }
    private bool HasAnswer { get; init; }
    private TaskbarAnchors? LastAnswer { get; init; }

    /// <summary>
    /// True when nothing has been asked yet, a foreground switch came since the last question, the last answers
    /// have not agreed yet, or <see cref="SlowCheck"/> has passed since the last question. A question whose
    /// answer never comes is therefore asked again after the slow check. A clock that went back is not due.
    /// </summary>
    public bool Due(TimeSpan now) =>
        !HasAsked || SwitchPending || Unsettled || now - LastAsked >= SlowCheck;

    /// <summary>A question went out at <paramref name="now"/>: it covers the switches seen so far and is the one that settles the answers.</summary>
    public TaskbarQuerySchedule Asked(TimeSpan now) =>
        this with { HasAsked = true, LastAsked = now, SwitchPending = false, Unsettled = false };

    /// <summary>
    /// The answer to a question; null when the taskbar could not be read, which is an answer like any other.
    /// The first answer, or one that differs from the answer before it, calls for another question.
    /// </summary>
    public TaskbarQuerySchedule Answered(TaskbarAnchors? answer) =>
        this with { Unsettled = !HasAnswer || answer != LastAnswer, HasAnswer = true, LastAnswer = answer };

    /// <summary>The foreground window changed: the next tick asks.</summary>
    public TaskbarQuerySchedule ForegroundSwitched() => this with { SwitchPending = true };
}
