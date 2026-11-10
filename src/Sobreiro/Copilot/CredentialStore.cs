using System.IO;
using System.Runtime.InteropServices;
using Sobreiro.Core.Copilot;

namespace Sobreiro.Copilot;

/// <summary>
/// Reads the Copilot CLI's sign-in from Windows Credential Manager. Target names are never logged or returned, and
/// each blob copy is zeroed after use.
/// </summary>
internal static class CredentialStore
{
    private const int ErrorNotFound = 1168;
    private const uint CredTypeGeneric = 1;

    [StructLayout(LayoutKind.Sequential)]
    private struct Credential
    {
        public uint Flags;
        public uint Type;
        public IntPtr TargetName;
        public IntPtr Comment;
        public long LastWritten;
        public uint CredentialBlobSize;
        public IntPtr CredentialBlob;
        public uint Persist;
        public uint AttributeCount;
        public IntPtr Attributes;
        public IntPtr TargetAlias;
        public IntPtr UserName;
    }

    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CredEnumerateW(string? filter, uint flags, out uint count, out IntPtr credentials);

    [DllImport("advapi32.dll")]
    private static extern void CredFree(IntPtr buffer);

    public static TokenSearch CopilotTokens()
    {
        var tokens = new List<string>();
        var entryFound = false;
        if (!CredEnumerateW("copilot-cli*", 0, out var count, out var list))
        {
            if (Marshal.GetLastWin32Error() != ErrorNotFound) return tokens;
            // No target name starts with copilot-cli. Older versions of the CLI may name it differently, so list
            // everything and match here: but only on a PC where the Copilot CLI has left its folder.
            if (!CopilotCliHasBeenUsed()) return tokens;
            if (!CredEnumerateW(null, 0, out count, out list)) return tokens;
        }

        try
        {
            for (var i = 0; i < count; i++)
            {
                var entry = Marshal.ReadIntPtr(list, i * IntPtr.Size);
                var credential = Marshal.PtrToStructure<Credential>(entry);
                if (credential.Type != CredTypeGeneric) continue;
                var name = Marshal.PtrToStringUni(credential.TargetName);
                if (name is null || !name.Contains("copilot-cli", StringComparison.OrdinalIgnoreCase)) continue;
                entryFound = true;
                if (credential.CredentialBlob == IntPtr.Zero || credential.CredentialBlobSize == 0) continue;

                var blob = new byte[credential.CredentialBlobSize];
                try
                {
                    Marshal.Copy(credential.CredentialBlob, blob, 0, blob.Length);
                    if (CopilotToken.FromBlob(blob) is { } token && !tokens.Contains(token)) tokens.Add(token);
                }
                finally
                {
                    Array.Clear(blob);
                }
            }
        }
        finally
        {
            CredFree(list);
        }
        return new TokenSearch(tokens, entryFound);
    }

    private static bool CopilotCliHasBeenUsed()
    {
        static string Folder(string variable, Environment.SpecialFolder fallback)
        {
            var value = Environment.GetEnvironmentVariable(variable);
            return string.IsNullOrEmpty(value) ? Environment.GetFolderPath(fallback) : value;
        }
        return Directory.Exists(Path.Combine(Folder("LOCALAPPDATA", Environment.SpecialFolder.LocalApplicationData), "copilot"))
            || Directory.Exists(Path.Combine(Folder("USERPROFILE", Environment.SpecialFolder.UserProfile), ".copilot"));
    }
}
