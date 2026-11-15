using System.IO;
using System.Runtime.InteropServices;
using Sobreiro.Core.Copilot;

namespace Sobreiro.Copilot;

/// <summary>
/// Reads the Copilot CLI's sign-in from Windows Credential Manager. Target names are never logged or returned,
/// each blob copy is zeroed after use, and the decoded text is cleared by <see cref="CopilotToken.FromBlob(ReadOnlySpan{byte})"/>.
/// </summary>
internal static class CredentialStore
{
    private const uint CredTypeGeneric = 1;

    // Fixed text: nothing read from Credential Manager is ever in an exception.
    private const string ListingFailed = "Credential Manager did not list its entries.";

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
    /// The Copilot CLI's tokens. Throws when Credential Manager refuses a listing, so the poll logs "error" and
    /// not "none": a refusal is not the same as finding nothing.
    /// </summary>
    public static TokenSearch CopilotTokens()
    {
        var tokens = new List<string>();
        var entryFound = false;
        if (!CredEnumerateW(CopilotCliCredential.Filter, 0, out var count, out var list))
        {
            switch (CopilotCliCredential.AfterFailure(Marshal.GetLastWin32Error(), CopilotCliHasBeenUsed()))
            {
                case ListingAnswer.NoEntry:
                    return tokens;
                case ListingAnswer.ListEverything:
                    // No target name starts with copilot-cli. Older versions of the CLI may name it differently,
                    // so list everything and match here.
                    if (CredEnumerateW(null, 0, out count, out list)) break;
                    if (CopilotCliCredential.AfterListingEverythingFailed(Marshal.GetLastWin32Error()) == ListingAnswer.NoEntry)
                        return tokens;
                    throw new InvalidOperationException(ListingFailed);
                default:
                    throw new InvalidOperationException(ListingFailed);
            }
        }

        try
        {
            for (var i = 0; i < count; i++)
            {
                var entry = Marshal.ReadIntPtr(list, i * IntPtr.Size);
                var credential = Marshal.PtrToStructure<Credential>(entry);
                if (credential.Type != CredTypeGeneric) continue;
                if (!CopilotCliCredential.IsOurs(Marshal.PtrToStringUni(credential.TargetName))) continue;
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
