using System.IO;
using System.Runtime.InteropServices;
using Sobreiro.Core.Copilot;

namespace Sobreiro.Copilot;

/// <summary>
/// Reads a GitHub sign-in from Windows Credential Manager: the Copilot CLI's entry, or the Copilot language
/// server's. Target names are never logged or returned, and each blob copy is zeroed after use.
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

    /// <summary>
    /// The Copilot CLI's sign-in. When no target name starts with its name, older versions of the CLI may name
    /// it differently, so every entry is listed and matched here: but only on a PC where the CLI has left its folder.
    /// </summary>
    public static TokenSearch CopilotTokens() =>
        Tokens(CopilotCredentialName.CliFilter, CopilotCredentialName.IsCli, listAllWhenNotFound: CopilotCliHasBeenUsed);

    /// <summary>The Copilot language server's sign-in, asked for by name. The other entries are never listed for it.</summary>
    public static TokenSearch LanguageServerTokens() =>
        Tokens(CopilotCredentialName.LanguageServerFilter, CopilotCredentialName.IsLanguageServer, listAllWhenNotFound: null);

    private static TokenSearch Tokens(string filter, Func<string?, bool> isOurs, Func<bool>? listAllWhenNotFound)
    {
        var tokens = new List<string>();
        var entryFound = false;
        if (!CredEnumerateW(filter, 0, out var count, out var list))
        {
            if (Marshal.GetLastWin32Error() != ErrorNotFound) return tokens;
            if (listAllWhenNotFound is null || !listAllWhenNotFound()) return tokens;
            if (!CredEnumerateW(null, 0, out count, out list)) return tokens;
        }

        try
        {
            for (var i = 0; i < count; i++)
            {
                var entry = Marshal.ReadIntPtr(list, i * IntPtr.Size);
                var credential = Marshal.PtrToStructure<Credential>(entry);
                if (credential.Type != CredTypeGeneric) continue;
                if (!isOurs(Marshal.PtrToStringUni(credential.TargetName))) continue;
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
