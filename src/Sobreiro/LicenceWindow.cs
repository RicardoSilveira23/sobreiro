using System.Drawing;
using System.IO;
using System.Windows.Forms;

namespace Sobreiro;

/// <summary>The licence texts carried inside Sobreiro.exe: Sobreiro's own (GPL-3.0) and the fonts'.</summary>
internal sealed class LicenceWindow : Form
{
    private LicenceWindow()
    {
        AutoScaleDimensions = new SizeF(96F, 96F);
        AutoScaleMode = AutoScaleMode.Dpi;
        Text = "Licences";
        ShowIcon = false;
        ShowInTaskbar = false;
        StartPosition = FormStartPosition.CenterScreen;
        ClientSize = new Size(560, 520);
        Controls.Add(new TextBox
        {
            Dock = DockStyle.Fill,
            Multiline = true,
            ReadOnly = true,
            ScrollBars = ScrollBars.Vertical,
            WordWrap = true,
            Font = SystemFonts.MessageBoxFont,
            Text = Read("LICENSE.txt") + "\r\n\r\n" + Read("OFL-Inter.txt") + "\r\n\r\n" + Read("OFL-Unbounded.txt"),
        });
    }

    public static void ShowLicences()
    {
        using var window = new LicenceWindow();
        window.ShowDialog();
    }

    private static string Read(string name)
    {
        using var stream = typeof(LicenceWindow).Assembly.GetManifestResourceStream(name);
        if (stream is null) return name + " is missing.";
        using var reader = new StreamReader(stream);
        return reader.ReadToEnd().Replace("\r\n", "\n").Replace("\n", "\r\n");
    }
}
