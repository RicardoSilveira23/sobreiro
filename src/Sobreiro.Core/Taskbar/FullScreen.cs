namespace Sobreiro.Core.Taskbar;

public static class FullScreen
{
    /// <summary>True for QUNS_BUSY (2), QUNS_RUNNING_D3D_FULL_SCREEN (3) and QUNS_PRESENTATION_MODE (4).</summary>
    public static bool IsFullScreen(int notificationState) => notificationState is 2 or 3 or 4;
}
