using Sobreiro.Core.Taskbar;

namespace Sobreiro.Tests;

public class TaskbarQueryScheduleTests
{
    private static readonly TaskbarAnchors A = new(2200, 900, 1392, 1440);
    private static readonly TaskbarAnchors B = new(2170, 900, 1392, 1440);
    private static TimeSpan S(double seconds) => TimeSpan.FromSeconds(seconds);
    private static TaskbarQuerySchedule Settled(TaskbarAnchors? answer, double askedAt) =>
        default(TaskbarQuerySchedule).Asked(S(askedAt)).Answered(answer).Asked(S(askedAt)).Answered(answer);

    [Fact] public void A_fresh_schedule_is_due() => Assert.True(default(TaskbarQuerySchedule).Due(S(0)));
    [Fact] public void A_question_that_is_out_waits_for_the_slow_check()
    {
        var asked = default(TaskbarQuerySchedule).Asked(S(0));
        Assert.False(asked.Due(S(2)));
        Assert.True(asked.Due(S(15)));
    }
    [Fact] public void One_answer_alone_asks_once_more() => Assert.True(default(TaskbarQuerySchedule).Asked(S(0)).Answered(A).Due(S(2)));
    [Fact] public void Two_answers_that_agree_settle_it() => Assert.False(Settled(A, 0).Due(S(2)));
    [Fact] public void One_answer_alone_then_waits_for_the_next() => Assert.False(default(TaskbarQuerySchedule).Asked(S(0)).Answered(A).Asked(S(2)).Due(S(4)));
    [Fact] public void Missing_answers_at_the_start_keep_asking()
    {
        var s = default(TaskbarQuerySchedule);
        for (var i = 0; i < 29; i++)
        {
            s = s.Asked(S(i * 2)).Answered(null);
            Assert.True(s.Due(S(i * 2 + 2)));
        }
    }
    [Fact] public void After_30_questions_without_anchors_it_settles()
    {
        var s = default(TaskbarQuerySchedule);
        for (var i = 0; i < 30; i++) s = s.Asked(S(i * 2)).Answered(null);
        Assert.False(s.Due(S(29 * 2 + 2)));
        Assert.True(s.Due(S(29 * 2 + 15)));
    }
    [Fact] public void Missing_answers_agree_once_anchors_were_seen() =>
        Assert.False(Settled(A, 0).Asked(S(16)).Answered(null).Asked(S(18)).Answered(null).Due(S(20)));
    [Fact] public void An_answer_that_differs_asks_again() => Assert.True(Settled(A, 0).Asked(S(16)).Answered(B).Due(S(18)));
    [Fact] public void An_answer_going_missing_differs() => Assert.True(Settled(A, 0).Asked(S(16)).Answered(null).Due(S(18)));
    [Fact] public void Answers_that_never_agree_keep_asking()
    {
        var s = default(TaskbarQuerySchedule);
        for (var i = 0; i < 50; i++)
        {
            s = s.Asked(S(i * 2)).Answered(i % 2 == 0 ? A : B);
            Assert.True(s.Due(S(i * 2 + 2)));
        }
    }
    [Theory, InlineData(14.9, false), InlineData(15, true), InlineData(16, true), InlineData(3600, true)]
    public void A_settled_schedule_checks_after_15_seconds(double later, bool due) =>
        Assert.Equal(due, Settled(A, 100).Due(S(100 + later)));
    [Fact] public void A_foreground_switch_asks_at_the_next_tick() => Assert.True(Settled(A, 0).ForegroundSwitched().Due(S(2)));
    [Fact] public void Many_switches_ask_twice()
    {
        var s = Settled(A, 0).ForegroundSwitched().ForegroundSwitched().ForegroundSwitched().Asked(S(2)).Answered(A);
        Assert.True(s.Due(S(4)));
        Assert.False(s.Asked(S(4)).Answered(A).Due(S(6)));
    }
    [Fact] public void A_switch_while_a_question_is_out_asks_again_after_the_answer()
    {
        var s = Settled(A, 0).Asked(S(16)).ForegroundSwitched().Answered(A);
        Assert.True(s.Due(S(18)));
        s = s.Asked(S(18)).Answered(A);
        Assert.True(s.Due(S(20)));
        Assert.False(s.Asked(S(20)).Answered(A).Due(S(22)));
    }
    [Fact] public void A_clock_that_went_back_is_not_due() => Assert.False(Settled(A, 100).Due(S(50)));
    [Fact] public void A_lost_answer_is_asked_again_after_15_seconds()
    {
        var asked = Settled(A, 0).Asked(S(16));
        Assert.False(asked.Due(S(18)));
        Assert.True(asked.Due(S(31)));
    }
    [Fact] public void The_slow_check_is_15_seconds() => Assert.Equal(S(15), TaskbarQuerySchedule.SlowCheck);
}
