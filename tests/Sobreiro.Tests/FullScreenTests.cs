using Sobreiro.Core.Taskbar;

namespace Sobreiro.Tests;

public class FullScreenTests
{
    [Theory, InlineData(2, true), InlineData(3, true), InlineData(4, true), InlineData(1, false), InlineData(5, false), InlineData(7, false)]
    public void Busy_d3d_and_presentation_are_full_screen(int state, bool full) =>
        Assert.Equal(full, FullScreen.IsFullScreen(state));
}
