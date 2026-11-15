using Sobreiro.Core.Taskbar;

namespace Sobreiro.Tests;

public class PillShowTests
{
    [Fact]
    public void Shown_in_place_above_the_taskbar_does_nothing() =>
        Assert.Equal(PillShowStep.None, PillShow.Step(visible: true, aboveTaskbar: true, inPlace: true));

    [Fact]
    public void Shown_above_the_taskbar_elsewhere_moves() =>
        Assert.Equal(PillShowStep.Move, PillShow.Step(visible: true, aboveTaskbar: true, inPlace: false));

    // After a full-screen window closes, the pill is shown again where it was, but Explorer raised the
    // taskbar past it while it was hidden: it is "shown, in place" and covered.
    [Theory, InlineData(true), InlineData(false)]
    public void Shown_below_the_taskbar_raises(bool inPlace) =>
        Assert.Equal(PillShowStep.Raise, PillShow.Step(visible: true, aboveTaskbar: false, inPlace));

    [Theory, InlineData(true, true), InlineData(true, false), InlineData(false, true), InlineData(false, false)]
    public void Hidden_raises(bool aboveTaskbar, bool inPlace) =>
        Assert.Equal(PillShowStep.Raise, PillShow.Step(visible: false, aboveTaskbar, inPlace));
}

public class ZOrderTests
{
    // Topmost first: 1 is the top window, 4 the bottom one.
    private static readonly nint[] Stack = [1, 2, 3, 4];

    private static nint Above(nint window)
    {
        var i = Array.IndexOf(Stack, window);
        return i > 0 ? Stack[i - 1] : 0;
    }

    [Fact]
    public void A_window_higher_in_the_stack_is_above() => Assert.True(ZOrder.IsAbove(2, 4, Above));

    [Fact]
    public void The_top_window_is_above() => Assert.True(ZOrder.IsAbove(1, 3, Above));

    [Fact]
    public void A_window_lower_in_the_stack_is_not_above() => Assert.False(ZOrder.IsAbove(4, 2, Above));

    [Fact]
    public void A_window_is_not_above_itself() => Assert.False(ZOrder.IsAbove(3, 3, Above));

    [Fact]
    public void A_window_not_in_the_stack_is_not_above() => Assert.False(ZOrder.IsAbove(9, 3, Above));

    // The stack can change during the walk; a loop must end.
    [Fact]
    public void A_cycle_ends() => Assert.False(ZOrder.IsAbove(9, 1, w => w == 1 ? 2 : 1));
}
