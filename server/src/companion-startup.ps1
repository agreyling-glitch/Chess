$ErrorActionPreference = 'Stop'
# Use the Unicode shell-link interface; WScript.Shell can lose characters in arguments.
Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
[ComImport, Guid("000214F9-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IronwoodShellLink {
    void GetPath(IntPtr path, int count, IntPtr data, uint flags);
    void GetIDList(out IntPtr list);
    void SetIDList(IntPtr list);
    void GetDescription([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder text, int count);
    void SetDescription([MarshalAs(UnmanagedType.LPWStr)] string text);
    void GetWorkingDirectory([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder text, int count);
    void SetWorkingDirectory([MarshalAs(UnmanagedType.LPWStr)] string text);
    void GetArguments([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder text, int count);
    void SetArguments([MarshalAs(UnmanagedType.LPWStr)] string text);
    void GetHotkey(out short key);
    void SetHotkey(short key);
    void GetShowCmd(out int command);
    void SetShowCmd(int command);
    void GetIconLocation([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder text, int count, out int index);
    void SetIconLocation([MarshalAs(UnmanagedType.LPWStr)] string text, int index);
    void SetRelativePath([MarshalAs(UnmanagedType.LPWStr)] string text, uint reserved);
    void Resolve(IntPtr window, uint flags);
    void SetPath([MarshalAs(UnmanagedType.LPWStr)] string text);
}
public static class IronwoodStartupLink {
    static object NewLink() {
        return Activator.CreateInstance(Type.GetTypeFromCLSID(new Guid("00021401-0000-0000-C000-000000000046")));
    }
    public static void Save(string path, string executable, string arguments) {
        object value = NewLink();
        try {
            var link = (IronwoodShellLink)value;
            link.SetPath(executable);
            link.SetArguments(arguments);
            link.SetWorkingDirectory(System.IO.Path.GetDirectoryName(executable));
            link.SetDescription("Start Ironwood Companion when I sign in");
            ((IPersistFile)value).Save(path, true);
        } finally { Marshal.FinalReleaseComObject(value); }
    }
    public static string Arguments(string path) {
        object value = NewLink();
        try {
            ((IPersistFile)value).Load(path, 0);
            var text = new StringBuilder(32768);
            ((IronwoodShellLink)value).GetArguments(text, text.Capacity);
            return text.ToString();
        } finally { Marshal.FinalReleaseComObject(value); }
    }
}
'@
if ($env:IRONWOOD_STARTUP_READ_TEST -eq '1') {
    [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding
    [Console]::Write([IronwoodStartupLink]::Arguments($env:IRONWOOD_STARTUP_LINK))
} else {
    [IronwoodStartupLink]::Save($env:IRONWOOD_STARTUP_LINK, $env:IRONWOOD_STARTUP_EXE, $env:IRONWOOD_STARTUP_ARGS)
}
