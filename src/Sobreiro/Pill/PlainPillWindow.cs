using System.Drawing;
using System.Runtime.InteropServices;
using Sobreiro.Core.Colours;
using Sobreiro.Core.Display;
using Sobreiro.Core.Taskbar;

namespace Sobreiro.Pill;

/// <summary>SPIKE (shell candidate F): what the tracker needs from a pill window, so the WPF one and the plain one can be swapped.</summary>
internal interface IPillWindow
{
    IntPtr Handle { get; }
    (int Width, int Height) PixelSize { get; }
    event Action? LeftClicked;
    event Action? RightClicked;
    void Render(PillModel model, TaskbarColours colours, string tooltip, int dpi);
    void SetDpi(int dpi);
    void MoveTo(int x, int y, int bandTop);
    void HideNow();
    void Close();
}

/// <summary>SPIKE: a plain window of a class registered here, with a message handler. No toolkit under it.</summary>
internal class PlainWindow
{
    private static readonly Dictionary<IntPtr, PlainWindow> Windows = [];
    private static readonly WndProc Proc = Dispatch;
    private static bool _registered;
    private const string ClassName = "SobreiroPlainWindow";

    public IntPtr Handle { get; private set; }

    protected void Create(uint exStyle, uint style, string title, IntPtr owner)
    {
        if (!_registered)
        {
            var cls = new WndClassEx
            {
                Size = (uint)Marshal.SizeOf<WndClassEx>(),
                Proc = Marshal.GetFunctionPointerForDelegate(Proc),
                Instance = GetModuleHandle(null),
                Cursor = LoadCursor(IntPtr.Zero, new IntPtr(32512)),
                ClassName = ClassName,
            };
            if (RegisterClassEx(ref cls) == 0) throw new InvalidOperationException("RegisterClassEx failed: " + Marshal.GetLastWin32Error());
            _registered = true;
        }
        Handle = CreateWindowEx(exStyle, ClassName, title, style, -32000, -32000, 1, 1, owner, IntPtr.Zero, GetModuleHandle(null), IntPtr.Zero);
        if (Handle == IntPtr.Zero) throw new InvalidOperationException("CreateWindowEx failed: " + Marshal.GetLastWin32Error());
        Windows[Handle] = this;
    }

    public virtual void Close()
    {
        if (Handle != IntPtr.Zero) DestroyWindow(Handle);
    }

    /// <summary>Null leaves the message to the system.</summary>
    protected virtual IntPtr? OnMessage(uint msg, IntPtr w, IntPtr l) => null;

    private static IntPtr Dispatch(IntPtr hwnd, uint msg, IntPtr w, IntPtr l)
    {
        if (Windows.TryGetValue(hwnd, out var window))
        {
            IntPtr? answer = null;
            try
            {
                answer = window.OnMessage(msg, w, l);
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"Sobreiro: plain window message failed: {ex.GetType().Name}");
            }
            if (msg == 0x82 /* WM_NCDESTROY */)
            {
                Windows.Remove(hwnd);
                window.Handle = IntPtr.Zero;
            }
            if (answer is { } value) return value;
        }
        return DefWindowProc(hwnd, msg, w, l);
    }

    private delegate IntPtr WndProc(IntPtr hwnd, uint msg, IntPtr w, IntPtr l);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct WndClassEx
    {
        public uint Size, Style;
        public IntPtr Proc;
        public int ClsExtra, WndExtra;
        public IntPtr Instance, Icon, Cursor, Background;
        public string? MenuName;
        public string ClassName;
        public IntPtr IconSm;
    }

    [DllImport("user32.dll", EntryPoint = "RegisterClassExW", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern ushort RegisterClassEx(ref WndClassEx cls);

    [DllImport("user32.dll", EntryPoint = "CreateWindowExW", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateWindowEx(uint exStyle, string className, string? title, uint style, int x, int y, int w, int h,
        IntPtr parent, IntPtr menu, IntPtr instance, IntPtr param);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool DestroyWindow(IntPtr hwnd);

    [DllImport("user32.dll", EntryPoint = "DefWindowProcW")]
    private static extern IntPtr DefWindowProc(IntPtr hwnd, uint msg, IntPtr w, IntPtr l);

    [DllImport("user32.dll", EntryPoint = "LoadCursorW")]
    private static extern IntPtr LoadCursor(IntPtr instance, IntPtr name);

    [DllImport("kernel32.dll", EntryPoint = "GetModuleHandleW", CharSet = CharSet.Unicode)]
    private static extern IntPtr GetModuleHandle(string? name);
}

/// <summary>SPIKE: the hidden top-level window that hears broadcasts such as TaskbarCreated.</summary>
internal sealed class PlainMessageWindow : PlainWindow
{
    private readonly Action<uint> _onMessage;

    public PlainMessageWindow(Action<uint> onMessage)
    {
        _onMessage = onMessage;
        Create(0, 0x80000000 /* WS_POPUP */, "SobreiroPillMessages", IntPtr.Zero);
    }

    protected override IntPtr? OnMessage(uint msg, IntPtr w, IntPtr l)
    {
        _onMessage(msg);
        return null;
    }
}

/// <summary>
/// SPIKE (shell candidate F): the pill on a plain layered window. Owned by the taskbar, never takes focus,
/// placed in physical pixels by the tracker; its picture comes from <see cref="PillPainter"/>.
/// </summary>
internal sealed class PlainPillWindow : PlainWindow, IPillWindow
{
    private readonly IntPtr _owner;
    private IntPtr _tip;
    private PillModel? _model;
    private TaskbarColours? _colours;
    private string _tooltip = "";
    private int _dpi;
    private bool _hover;
    private (int Width, int Height) _size;

    public PlainPillWindow(IntPtr owner)
    {
        _owner = owner;
        Create(WsExLayered | WsExToolWindow | WsExNoActivate | WsExTopmost, WsPopup, "Sobreiro pill", owner);
    }

    public event Action? LeftClicked;

    public event Action? RightClicked;

    public (int Width, int Height) PixelSize => _size;

    public void Render(PillModel model, TaskbarColours colours, string tooltip, int dpi)
    {
        if (tooltip != _tooltip || _tip == IntPtr.Zero) SetTooltip(tooltip);
        if (model == _model && colours == _colours && dpi == _dpi) return;
        _model = model;
        _colours = colours;
        _dpi = dpi;
        Paint();
    }

    public void SetDpi(int dpi)
    {
        if (dpi == _dpi || _model is null) return;
        _dpi = dpi;
        Paint();
    }

    public void MoveTo(int x, int y, int bandTop)
    {
        // Same steps as the WPF window: raise a hidden pill or one under the taskbar, else only move.
        var hwnd = Handle;
        var visible = IsWindowVisible(hwnd);
        var inPlace = GetWindowRect(hwnd, out var r) && r.Left == x && r.Top == y;
        var aboveTaskbar = ZOrder.IsAbove(hwnd, _owner, w => GetWindow(w, GwHwndPrev));
        switch (PillShow.Step(visible, aboveTaskbar, inPlace))
        {
            case PillShowStep.Raise:
                SetWindowPos(hwnd, HwndTopmost, x, y, 0, 0, SwpNoOwnerZOrder | SwpNoSize | SwpNoActivate | SwpShowWindow);
                break;
            case PillShowStep.Move:
                SetWindowPos(hwnd, IntPtr.Zero, x, y, 0, 0, SwpNoZOrder | SwpNoOwnerZOrder | SwpNoSize | SwpNoActivate | SwpShowWindow);
                break;
        }
    }

    public void HideNow() => ShowWindow(Handle, SwHide);

    protected override IntPtr? OnMessage(uint msg, IntPtr w, IntPtr l)
    {
        switch (msg)
        {
            case 0x21: // WM_MOUSEACTIVATE
                return new IntPtr(3); // MA_NOACTIVATE
            case 0x202: // WM_LBUTTONUP
                LeftClicked?.Invoke();
                return IntPtr.Zero;
            case 0x205: // WM_RBUTTONUP
                RightClicked?.Invoke();
                return IntPtr.Zero;
            case 0x200: // WM_MOUSEMOVE
                if (!_hover)
                {
                    _hover = true;
                    var track = new TrackMouseEventArgs { Size = (uint)Marshal.SizeOf<TrackMouseEventArgs>(), Flags = 2 /* TME_LEAVE */, Hwnd = Handle };
                    TrackMouseEvent(ref track);
                    Paint();
                }
                return null;
            case 0x2A3: // WM_MOUSELEAVE
                _hover = false;
                Paint();
                return null;
        }
        return null;
    }

    // Draws the face and hands the picture to the window manager; the window keeps its place.
    private void Paint()
    {
        if (Handle == IntPtr.Zero || _model is not { } model || _colours is not { } colours || _dpi <= 0) return;
        using var bitmap = PillPainter.Draw(model, colours, _dpi / 96.0, _hover);
        var screen = GetDC(IntPtr.Zero);
        var memory = CreateCompatibleDC(screen);
        var hbitmap = bitmap.GetHbitmap(Color.FromArgb(0));
        var old = SelectObject(memory, hbitmap);
        try
        {
            var size = new NativeSize { X = bitmap.Width, Y = bitmap.Height };
            var source = new NativeSize();
            var blend = new BlendFunction { Alpha = 255, Format = 1 /* AC_SRC_ALPHA */ };
            if (UpdateLayeredWindow(Handle, screen, IntPtr.Zero, ref size, memory, ref source, 0, ref blend, 2 /* ULW_ALPHA */))
                _size = (bitmap.Width, bitmap.Height);
        }
        finally
        {
            SelectObject(memory, old);
            DeleteObject(hbitmap);
            DeleteDC(memory);
            ReleaseDC(IntPtr.Zero, screen);
        }
    }

    // The system's own tooltip, shown for the whole window.
    private void SetTooltip(string text)
    {
        _tooltip = text;
        if (Handle == IntPtr.Zero) return;
        var created = _tip == IntPtr.Zero;
        if (created)
        {
            _tip = CreateWindowEx(WsExTopmost, "tooltips_class32", null, WsPopup | 0x01 /* TTS_ALWAYSTIP */ | 0x02 /* TTS_NOPREFIX */,
                0, 0, 0, 0, Handle, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);
            if (_tip == IntPtr.Zero) return;
            SendMessage(_tip, 0x418 /* TTM_SETMAXTIPWIDTH */, IntPtr.Zero, new IntPtr(400));
        }
        var textPtr = Marshal.StringToHGlobalUni(text);
        try
        {
            var tool = new ToolInfo { Size = 64, Flags = 0x01 /* TTF_IDISHWND */ | 0x10 /* TTF_SUBCLASS */, Hwnd = Handle, Id = Handle, Text = textPtr };
            SendMessage(_tip, created ? 0x432u /* TTM_ADDTOOLW */ : 0x439u /* TTM_UPDATETIPTEXTW */, IntPtr.Zero, ref tool);
        }
        finally
        {
            Marshal.FreeHGlobal(textPtr);
        }
    }

    private const uint WsPopup = 0x80000000;
    private const uint WsExLayered = 0x80000;
    private const uint WsExToolWindow = 0x80;
    private const uint WsExNoActivate = 0x08000000;
    private const uint WsExTopmost = 0x8;
    private const uint SwpNoSize = 0x1;
    private const uint SwpNoZOrder = 0x4;
    private const uint SwpNoActivate = 0x10;
    private const uint SwpNoOwnerZOrder = 0x200;
    private const uint SwpShowWindow = 0x40;
    private const int SwHide = 0;
    private const uint GwHwndPrev = 3;
    private static readonly IntPtr HwndTopmost = new(-1);

    [StructLayout(LayoutKind.Sequential)]
    private struct NativeRect
    {
        public int Left, Top, Right, Bottom;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct NativeSize
    {
        public int X, Y;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct BlendFunction
    {
        public byte Op, Flags, Alpha, Format;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct TrackMouseEventArgs
    {
        public uint Size, Flags;
        public IntPtr Hwnd;
        public uint HoverTime;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct ToolInfo
    {
        public uint Size, Flags;
        public IntPtr Hwnd, Id;
        public NativeRect Rect;
        public IntPtr Instance, Text, Param, Reserved;
    }

    [DllImport("user32.dll", EntryPoint = "CreateWindowExW", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateWindowEx(uint exStyle, string className, string? title, uint style, int x, int y, int w, int h,
        IntPtr parent, IntPtr menu, IntPtr instance, IntPtr param);

    [DllImport("user32.dll", EntryPoint = "SendMessageW")]
    private static extern IntPtr SendMessage(IntPtr hwnd, uint msg, IntPtr w, IntPtr l);

    [DllImport("user32.dll", EntryPoint = "SendMessageW")]
    private static extern IntPtr SendMessage(IntPtr hwnd, uint msg, IntPtr w, ref ToolInfo tool);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool TrackMouseEvent(ref TrackMouseEventArgs args);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool UpdateLayeredWindow(IntPtr hwnd, IntPtr screenDc, IntPtr position, ref NativeSize size, IntPtr sourceDc,
        ref NativeSize source, uint key, ref BlendFunction blend, uint flags);

    [DllImport("user32.dll")]
    private static extern IntPtr GetDC(IntPtr hwnd);

    [DllImport("user32.dll")]
    private static extern int ReleaseDC(IntPtr hwnd, IntPtr dc);

    [DllImport("gdi32.dll")]
    private static extern IntPtr CreateCompatibleDC(IntPtr dc);

    [DllImport("gdi32.dll")]
    private static extern IntPtr SelectObject(IntPtr dc, IntPtr obj);

    [DllImport("gdi32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool DeleteObject(IntPtr obj);

    [DllImport("gdi32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool DeleteDC(IntPtr dc);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindowVisible(IntPtr hwnd);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetWindowRect(IntPtr hwnd, out NativeRect rect);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowPos(IntPtr hwnd, IntPtr insertAfter, int x, int y, int cx, int cy, uint flags);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ShowWindow(IntPtr hwnd, int command);

    [DllImport("user32.dll")]
    private static extern IntPtr GetWindow(IntPtr hwnd, uint command);
}
