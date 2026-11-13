namespace Sobreiro.Core.Taskbar;

/// <summary>Decides whether a pill tick also asks UI Automation where the taskbar's parts are.</summary>
public readonly record struct TaskbarQuerySchedule
{
    /// <summary>How long a settled schedule goes without asking: layout changes that raise no foreground switch are noticed within this.</summary>
    public static readonly TimeSpan SlowCheck = TimeSpan.FromSeconds(15);

    // default(TaskbarQuerySchedule) is the fresh schedule: nothing asked, nothing answered.
    private bool HasAsked { get; init; }
    private TimeSpan LastAsked { get; init; }
    private bool Out { get; init; }
    private bool SwitchPending { get; init; }
    private int AnswerCount { get; init; }
    private TaskbarAnchors? Latest { get; init; }
    private TaskbarAnchors? Before { get; init; }

    /// <summary>
    /// True until two answers in a row agree (a missing answer counts as one), and again after a foreground
    /// switch or <see cref="SlowCheck"/>. Never while a question is still out. <see cref="Asked"/> clears the
    /// switch and records the time, and a clock that went back is simply not due.
    /// </summary>
    public bool Due(TimeSpan now) =>
        !Out && (!HasAsked
        || SwitchPending
        || AnswerCount < 2
        || Latest != Before
        || now - LastAsked >= SlowCheck);

    /// <summary>A question went out at <paramref name="now"/>; the switches seen so far are covered by it.</summary>
    public TaskbarQuerySchedule Asked(TimeSpan now) => this with { HasAsked = true, Out = true, LastAsked = now, SwitchPending = false };

    /// <summary>The answer to the last question; null when the taskbar could not be read.</summary>
    public TaskbarQuerySchedule Answered(TaskbarAnchors? answer) =>
        this with { Out = false, AnswerCount = Math.Min(AnswerCount + 1, 2), Before = Latest, Latest = answer };

    /// <summary>The foreground window changed: the next tick asks.</summary>
    public TaskbarQuerySchedule ForegroundSwitched() => this with { SwitchPending = true };
}
