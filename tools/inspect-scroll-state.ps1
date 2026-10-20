param([int]$ProcessId, [int]$Seconds=30)
$ErrorActionPreference='Stop'
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class SnapZyScrollInspect {
 public delegate bool EnumProc(IntPtr h,IntPtr p);
 [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f,IntPtr p);
 [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h,EnumProc f,IntPtr p);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] public static extern bool IsHungAppWindow(IntPtr h);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h,StringBuilder b,int n);
 [DllImport("user32.dll",CharSet=CharSet.Unicode,EntryPoint="SendMessageTimeoutW")] public static extern IntPtr ReadText(IntPtr h,uint m,IntPtr n,StringBuilder b,uint flags,uint timeout,out IntPtr result);
 public static string Text(IntPtr h){var b=new StringBuilder(1000);IntPtr r;ReadText(h,13,new IntPtr(1000),b,2,100,out r);return b.ToString();}
 public static string Class(IntPtr h){var b=new StringBuilder(256);GetClassName(h,b,256);return b.ToString();}
}
'@
if(-not $ProcessId){
    $candidates=@(Get-CimInstance Win32_Process -Filter "Name='SnapZy-Rust.exe' OR Name='snapzy-rust.exe'" | Where-Object {$_.CommandLine -notmatch '--scroll-detect-worker'})
    if($candidates.Count -ne 1){throw 'Supply the PID of the Rust preview to inspect.'}
    $ProcessId=$candidates[0].ProcessId
}
$last=''
$clock=[Diagnostics.Stopwatch]::StartNew()
do {
    $script:states=[Collections.Generic.List[object]]::new()
    [SnapZyScrollInspect]::EnumWindows({param($h,$p)
        $owner=0;[SnapZyScrollInspect]::GetWindowThreadProcessId($h,[ref]$owner) | Out-Null
        if($owner -eq $ProcessId -and [SnapZyScrollInspect]::IsWindowVisible($h)){
            $class=[SnapZyScrollInspect]::Class($h)
            if($class -in @('SnapZyRustPreview.Scroll','SnapZyRustPreview.Selection','SnapZyRustPreview.Launcher')){
                $script:states.Add([pscustomobject]@{Handle=$h.ToInt64();Class=$class;Hung=[SnapZyScrollInspect]::IsHungAppWindow($h)})
                [SnapZyScrollInspect]::EnumChildWindows($h,{param($c,$p)
                    $kind=[SnapZyScrollInspect]::Class($c)
                    if($kind -in @('Static','Button')){$script:states.Add([pscustomobject]@{Class=$kind;Text=[SnapZyScrollInspect]::Text($c)})}
                    return $true
                },[IntPtr]::Zero) | Out-Null
            }
        }
        return $true
    },[IntPtr]::Zero) | Out-Null
    $state=ConvertTo-Json -InputObject $script:states.ToArray() -Compress
    if($state -ne $last){[pscustomobject]@{ElapsedMs=[int]$clock.ElapsedMilliseconds;ProcessId=$ProcessId;State=$script:states.ToArray()} | ConvertTo-Json -Depth 5 -Compress;$last=$state}
    Start-Sleep -Milliseconds 500
}while($clock.Elapsed.TotalSeconds -lt [Math]::Clamp($Seconds,1,120))
