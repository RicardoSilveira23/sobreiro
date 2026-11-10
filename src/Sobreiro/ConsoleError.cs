using System.Runtime.InteropServices;

namespace Sobreiro;

/// <summary>Says something on stderr from a WinExe, which has no console of its own.</summary>
internal static class ConsoleError
{
    /// <summary>Writes the message, borrowing the parent's console when stderr is not already redirected.</summary>
    public static void Write(string message)
    {
        if (GetStdHandle(StdErrorHandle) == IntPtr.Zero) AttachConsole(AttachParentProcess);
        Console.Error.WriteLine(message);
    }

    /// <summary>Writes the message and returns the exit code.</summary>
    public static int Fail(string message, int code)
    {
        Write(message);
        return code;
    }

    private const int StdErrorHandle = -12;
    private const int AttachParentProcess = -1;

    [DllImport("kernel32.dll")]
    private static extern IntPtr GetStdHandle(int handle);

    [DllImport("kernel32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool AttachConsole(int processId);
}
