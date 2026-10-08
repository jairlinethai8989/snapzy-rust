param([string]$Executable = (Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-msvc\release\snapzy-rust.exe'))
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Windows.Forms,System.Drawing
Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
namespace SnapZyTests {
 public static class Native {
  public delegate bool EnumProc(IntPtr h,IntPtr p);
  [StructLayout(LayoutKind.Sequential)] public struct Rect {public int Left,Top,Right,Bottom;}
  [StructLayout(LayoutKind.Sequential)] public struct Point {public int X,Y;}
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc callback,IntPtr p);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent,EnumProc callback,IntPtr p);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h,StringBuilder b,int n);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h,StringBuilder b,int n);
  [DllImport("user32.dll",CharSet=CharSet.Unicode,EntryPoint="SendMessageW")] public static extern IntPtr ReadText(IntPtr h,uint m,IntPtr n,StringBuilder b);
  [DllImport("user32.dll",CharSet=CharSet.Unicode,EntryPoint="SendMessageW")] public static extern IntPtr WriteText(IntPtr h,uint m,IntPtr n,string text);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern bool SetWindowText(IntPtr h,string text);
  [DllImport("user32.dll",CharSet=CharSet.Unicode,EntryPoint="FindWindowW")] public static extern IntPtr FindWindow(string cls,string title);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h,uint m,IntPtr w,IntPtr l);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h,uint m,IntPtr w,IntPtr l);
  [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr h,int id);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h,out Rect r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h,out Rect r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h,ref Point p);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h,IntPtr after,int x,int y,int w,int height,uint flags);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h,int command);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool UpdateWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetDC(IntPtr h);
  [DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr h,IntPtr dc);
  [DllImport("gdi32.dll")] public static extern bool BitBlt(IntPtr target,int x,int y,int w,int height,IntPtr source,int sx,int sy,uint rop);
  [DllImport("gdi32.dll")] public static extern bool GdiFlush();
  [DllImport("dwmapi.dll")] public static extern int DwmFlush();
  [DllImport("user32.dll")] public static extern int GetSystemMetrics(int id);
  [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
  [DllImport("user32.dll")] public static extern IntPtr GetWindowDpiAwarenessContext(IntPtr window);
  [DllImport("user32.dll")] public static extern int GetAwarenessFromDpiAwarenessContext(IntPtr context);
  [DllImport("user32.dll")] public static extern bool RegisterHotKey(IntPtr h,int id,uint modifiers,uint key);
  [DllImport("user32.dll")] public static extern bool UnregisterHotKey(IntPtr h,int id);
  [DllImport("user32.dll")] public static extern uint GetGuiResources(IntPtr process,uint flags);
  public static string Class(IntPtr h) {var b=new StringBuilder(128);GetClassName(h,b,b.Capacity);return b.ToString();}
  public static string Text(IntPtr h) {var b=new StringBuilder(32768);ReadText(h,0x000d,(IntPtr)b.Capacity,b);return b.ToString();}
  public static IntPtr[] Windows(int process,string type) {
   var found=new List<IntPtr>();
   EnumWindows((h,p)=>{uint id;GetWindowThreadProcessId(h,out id);if(id==process && Class(h)==type)found.Add(h);return true;},IntPtr.Zero);
   return found.ToArray();
  }
  public static IntPtr[] Children(IntPtr h) {var found=new List<IntPtr>();EnumChildWindows(h,(c,p)=>{found.Add(c);return true;},IntPtr.Zero);return found.ToArray();}
  public static string ProcessWindows(int process) {
   var found=new List<string>();
   EnumWindows((h,p)=>{uint id;GetWindowThreadProcessId(h,out id);if(id==process){var b=new StringBuilder(512);GetWindowText(h,b,b.Capacity);found.Add(Class(h)+" title='"+b+"' id1="+(GetDlgItem(h,1)!=IntPtr.Zero)+" id6="+(GetDlgItem(h,6)!=IntPtr.Zero));}return true;},IntPtr.Zero);
   return String.Join(" | ",found);
  }
  public static Rect Client(IntPtr h) {Rect r;GetClientRect(h,out r);var p=new Point();ClientToScreen(h,ref p);r.Right+=p.X;r.Bottom+=p.Y;r.Left=p.X;r.Top=p.Y;return r;}
 }
}
'@
$root=Split-Path $PSScriptRoot
$runDirectory=Join-Path $root ('artifacts\smoke-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $runDirectory -Force | Out-Null
$Executable=(Resolve-Path -LiteralPath $Executable).Path
$process=$null
$reserved=$false
$fixture=$null
$oldDpi=[SnapZyTests.Native]::SetThreadDpiAwarenessContext([IntPtr](-4))
$measurements=[ordered]@{}
function Wait-For([scriptblock]$Condition,[string]$Failure,[int]$Timeout=5000) {
    $watch=[Diagnostics.Stopwatch]::StartNew()
    while ($watch.ElapsedMilliseconds -lt $Timeout) {
        [Windows.Forms.Application]::DoEvents()
        $value=& $Condition
        if ($value) {return $value}
        if ($process -and $process.HasExited) {throw "App exited during test: $Failure"}
        Start-Sleep -Milliseconds 10
    }
    if ($Failure -eq 'Conflict warning did not appear.') {throw "$Failure Windows: $([SnapZyTests.Native]::ProcessWindows($process.Id))"}
    throw $Failure
}
function Window-Of([string]$Class) {
    $windows=[SnapZyTests.Native]::Windows($process.Id,$Class)
    if ($windows.Length) {
        $window=$windows[0]
        if ($Class -eq 'SnapZyRustPreview.Capture' -and [SnapZyTests.Native]::GetDlgItem($window,401) -eq [IntPtr]::Zero) {return $null}
        if ($Class -eq 'SnapZyRustPreview.Hotkeys' -and [SnapZyTests.Native]::GetDlgItem($window,301) -eq [IntPtr]::Zero) {return $null}
        if ($Class -eq '#32770' -and [SnapZyTests.Native]::GetDlgItem($window,1) -eq [IntPtr]::Zero -and [SnapZyTests.Native]::GetDlgItem($window,6) -eq [IntPtr]::Zero -and [SnapZyTests.Native]::Text($window) -ne 'SnapZy Rust Preview 0.1.0-alpha.1') {return $null}
        return $window
    }
    if ($Class -eq '#32770') {
        $dialog=[SnapZyTests.Native]::FindWindow('#32770','SnapZy Rust Preview 0.1.0-alpha.1')
        if ($dialog -ne [IntPtr]::Zero) {return $dialog}
    }
    return $null
}
function Dismiss-Message([IntPtr]$Window,[int]$Id) {
    [SnapZyTests.Native]::PostMessage($Window,0x0010,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
    Wait-For {-not [SnapZyTests.Native]::IsWindowVisible($Window)} 'Message box did not close.' | Out-Null
}
function Click([IntPtr]$Parent,[int]$Id) {
    $control=Wait-For {
        $candidate=[SnapZyTests.Native]::GetDlgItem($Parent,$Id)
        if ($candidate -ne [IntPtr]::Zero) {return $candidate}
        return $null
    } "Control $Id is not ready."
    [SnapZyTests.Native]::PostMessage($Parent,0x0111,[IntPtr]$Id,$control) | Out-Null
}
function Close-Preview([IntPtr]$Preview) {
    [SnapZyTests.Native]::PostMessage($Preview,0x10,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
    $dialog=Wait-For {Window-Of '#32770'} 'Unsaved capture confirmation did not appear.'
    Click $dialog 6
    Wait-For {-not (Window-Of 'SnapZyRustPreview.Capture')} 'Preview did not close.' | Out-Null
}
function Mouse([IntPtr]$Overlay,[uint32]$Message,[int]$X,[int]$Y) {
    $parameter=([int64]($Y-band 0xffff)-shl 16)-bor ($X-band 0xffff)
    [SnapZyTests.Native]::PostMessage($Overlay,$Message,[IntPtr](1),[IntPtr]$parameter) | Out-Null
}
function Begin-Area {
    [SnapZyTests.Native]::SetWindowPos($fixture.Handle,[IntPtr](-1),0,0,0,0,3) | Out-Null
    [SnapZyTests.Native]::ShowWindow($fixture.Handle,5) | Out-Null
    [SnapZyTests.Native]::UpdateWindow($fixture.Handle) | Out-Null
    [Windows.Forms.Application]::DoEvents()
    [SnapZyTests.Native]::DwmFlush() | Out-Null
    $bounds=[SnapZyTests.Native]::Client($fixture.Handle)
    $sample=[Drawing.Bitmap]::new(1,1)
    $drawing=[Drawing.Graphics]::FromImage($sample)
    try {
        $drawing.CopyFromScreen($bounds.Left+80,$bounds.Top+80,0,0,$sample.Size)
        $color=$sample.GetPixel(0,0)
        $awareness=[SnapZyTests.Native]::GetAwarenessFromDpiAwarenessContext([SnapZyTests.Native]::GetWindowDpiAwarenessContext($fixture.Handle))
        if ($color.ToArgb() -ne $fixture.BackColor.ToArgb()) {
            throw "Fixture source is covered: bounds=$($bounds.Left),$($bounds.Top),$($bounds.Right),$($bounds.Bottom); dpi=$awareness; pixel=$color"
        }
    } finally {$drawing.Dispose();$sample.Dispose()}
    $clock=[Diagnostics.Stopwatch]::StartNew()
    Click $main 101
    $overlay=Wait-For {Window-Of 'SnapZyRustPreview.Selection'} 'Area selection did not appear.'
    return @{Window=$overlay;Milliseconds=$clock.Elapsed.TotalMilliseconds}
}
try {
    $fixture=[Windows.Forms.Form]::new()
    $fixture.Text='SnapZy Rust test fixture'
    $fixture.ShowInTaskbar=$false;$fixture.TopMost=$true
    $fixture.AutoScaleMode=[Windows.Forms.AutoScaleMode]::None
    $fixture.ClientSize=[Drawing.Size]::new(320,180)
    $fixture.BackColor=[Drawing.Color]::FromArgb(204,69,119)
    $fixture.StartPosition=[Windows.Forms.FormStartPosition]::Manual
    $screen=[Windows.Forms.Screen]::PrimaryScreen.WorkingArea
    $fixture.Location=[Drawing.Point]::new($screen.Left+650,$screen.Top+100)
    $fixture.Show();[Windows.Forms.Application]::DoEvents()
    $clock=[Diagnostics.Stopwatch]::StartNew()
    $process=Start-Process -FilePath $Executable -ArgumentList @('--test-data-dir',"`"$runDirectory`"") -WindowStyle Hidden -PassThru
    $main=Wait-For {Window-Of 'SnapZyRustPreview.Launcher'} 'Launcher did not initialize.'
    Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($main,401)) -eq 'Ready'} 'Launcher controls did not become ready.' | Out-Null
    Wait-For {
        foreach($log in Get-ChildItem (Join-Path $runDirectory 'Logs') -Filter 'performance-*.log' -ErrorAction SilentlyContinue) {
            if (Get-Content -LiteralPath $log.FullName | Where-Object {$_ -match ' launcher.ready '}) {return $true}
        }
        return $false
    } 'Launcher ready timing was not logged.' | Out-Null
    $measurements.launcher_window_ms=[Math]::Round($clock.Elapsed.TotalMilliseconds,2)
    [SnapZyTests.Native]::SetWindowPos($main,[IntPtr](-1),$screen.Left+20,$screen.Top+20,0,0,1) | Out-Null
    if ([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($main,104)) -ne 'EN') {throw 'English default was not applied.'}
    Click $main 104
    Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($main,104)) -eq 'TH'} 'Language did not switch to Thai.' | Out-Null
    Click $main 104
    Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($main,104)) -eq 'EN'} 'Language did not switch back.' | Out-Null
    $settings=Get-Content -LiteralPath (Join-Path $runDirectory 'settings.json') -Raw | ConvertFrom-Json
    if ($settings.language -ne 'en' -or $settings.hotkey_enabled) {throw 'Default hotkey should be opt-in and language should persist.'}

    $begin=Begin-Area
    $measurements.area_selection_window_ms=[Math]::Round($begin.Milliseconds,2)
    $overlay=$begin.Window
    [SnapZyTests.Native]::PostMessage($overlay,0x100,[IntPtr](27),[IntPtr]::Zero) | Out-Null
    Wait-For {-not (Window-Of 'SnapZyRustPreview.Selection')} 'Escape did not cancel selection.' | Out-Null
    if (Window-Of 'SnapZyRustPreview.Capture') {throw 'Escape created a capture.'}

    $begin=Begin-Area
    $overlay=$begin.Window
    for ($i=0;$i -lt 20;$i++) {[SnapZyTests.Native]::PostMessage($main,0x312,[IntPtr](1),[IntPtr]::Zero) | Out-Null}
    [Windows.Forms.Application]::DoEvents()
    $client=[SnapZyTests.Native]::Client($fixture.Handle)
    $virtualX=[SnapZyTests.Native]::GetSystemMetrics(76)
    $virtualY=[SnapZyTests.Native]::GetSystemMetrics(77)
    $x=$client.Left+20-$virtualX;$y=$client.Top+20-$virtualY
    Mouse $overlay 0x201 $x $y
    Mouse $overlay 0x200 ($x+160) ($y+80)
    $clock=[Diagnostics.Stopwatch]::StartNew()
    Mouse $overlay 0x202 ($x+160) ($y+80)
    $preview=Wait-For {Window-Of 'SnapZyRustPreview.Capture'} 'Area capture did not open preview.'
    $measurements.area_preview_window_ms=[Math]::Round($clock.Elapsed.TotalMilliseconds,2)
    $label=[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401))
    if ($label -ne '160 x 80 px') {throw "Area dimensions are wrong: $label"}
    if ([SnapZyTests.Native]::Windows($process.Id,'SnapZyRustPreview.Capture').Count -ne 1) {throw 'Repeated input created duplicate previews.'}
    Click $preview 202
    $dialog=Wait-For {Window-Of '#32770'} 'Save PNG dialog did not open.'
    $filename=Wait-For {
        foreach ($control in [SnapZyTests.Native]::Children($dialog)) {
            if ([SnapZyTests.Native]::Class($control) -eq 'Edit' -and [SnapZyTests.Native]::Text($control).Contains('SnapZy-Rust-')) {return $control}
        }
        return $null
    } 'Cannot locate the save filename control.'
    $png=Join-Path $runDirectory 'area-ไทย.png'
    [SnapZyTests.Native]::WriteText($filename,0x000c,[IntPtr]::Zero,$png) | Out-Null
    if ([SnapZyTests.Native]::Text($filename) -ne $png) {throw 'Save filename was not applied.'}
    Click $dialog 1
    Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -eq 'Saved'} 'PNG export did not complete.' | Out-Null
    $image=[Drawing.Bitmap]::FromFile($png)
    try {
        if ($image.Width -ne 160 -or $image.Height -ne 80 -or $image.GetPixel(0,0).ToArgb() -ne $fixture.BackColor.ToArgb()) {throw 'Export changed original dimensions or pixel colors.'}
    } finally {$image.Dispose()}
    [SnapZyTests.Native]::PostMessage($preview,0x10,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
    Wait-For {-not (Window-Of 'SnapZyRustPreview.Capture')} 'Saved preview did not close.' | Out-Null

    Click $main 102
    $overlay=Wait-For {Window-Of 'SnapZyRustPreview.Selection'} 'Window selection did not appear.'
    $client=[SnapZyTests.Native]::Client($fixture.Handle)
    $x=$client.Left+100-$virtualX;$y=$client.Top+80-$virtualY
    Mouse $overlay 0x200 $x $y
    Mouse $overlay 0x201 $x $y
    Mouse $overlay 0x202 $x $y
    $preview=Wait-For {Window-Of 'SnapZyRustPreview.Capture'} 'Window capture did not open preview.'
    $label=[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401))
    $diagnostics=Wait-For {
        $path=Join-Path $runDirectory 'Logs/capture-diagnostics.log'
        if (Test-Path $path) {
            $lines=Get-Content -LiteralPath $path
            $frameLine=$lines | Where-Object {$_ -match '^window-frame='} | Select-Object -Last 1
            if ($frameLine) {return $frameLine}
        }
        return $null
    } 'Window capture frame diagnostic was not written.'
    if ($diagnostics -notmatch '^window-frame=322x212 center-bgra=\[119, 69, 204, 255\]$') {
        throw "Window frame did not preserve the fixture source pixel: $diagnostics"
    }
    Close-Preview $preview

    Click $main 103
    $preview=Wait-For {Window-Of 'SnapZyRustPreview.Capture'} 'Desktop capture did not open preview.'
    $expected='{0} x {1} px' -f [SnapZyTests.Native]::GetSystemMetrics(78),[SnapZyTests.Native]::GetSystemMetrics(79)
    $label=[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401))
    if ($label -ne $expected) {throw "Desktop dimensions mismatch: $label vs $expected"}
    Close-Preview $preview

    Click $main 105
    $keys=Wait-For {Window-Of 'SnapZyRustPreview.Hotkeys'} 'Hotkey settings did not open.'
    $reserved=[SnapZyTests.Native]::RegisterHotKey([IntPtr]::Zero,9881,0x4007,122)
    if (-not $reserved) {throw 'Cannot reserve the isolated hotkey conflict fixture.'}
    [SnapZyTests.Native]::SendMessage([SnapZyTests.Native]::GetDlgItem($keys,302),0xf1,[IntPtr](1),[IntPtr]::Zero) | Out-Null
    [SnapZyTests.Native]::SendMessage([SnapZyTests.Native]::GetDlgItem($keys,303),0x14e,[IntPtr](3),[IntPtr]::Zero) | Out-Null
    [SnapZyTests.Native]::SendMessage([SnapZyTests.Native]::GetDlgItem($keys,304),0x14e,[IntPtr](3),[IntPtr]::Zero) | Out-Null
    Click $keys 301
    $dialog=Wait-For {Window-Of '#32770'} 'Conflict warning did not appear.'
    $messages=Get-Content -LiteralPath (Join-Path $runDirectory 'Logs/capture-diagnostics.log')
    if (-not ($messages | Where-Object {$_ -eq 'notification error=true text=This hotkey is used by another application. Choose a different combination.'})) {throw 'Hotkey conflict was not explained.'}
    Dismiss-Message $dialog 1
    $settings=Get-Content -LiteralPath (Join-Path $runDirectory 'settings.json') -Raw | ConvertFrom-Json
    if ($settings.hotkey_enabled) {throw 'Failed hotkey registration was incorrectly persisted.'}
    [SnapZyTests.Native]::UnregisterHotKey([IntPtr]::Zero,9881) | Out-Null;$reserved=$false
    Click $keys 301
    $dialog=Wait-For {Window-Of '#32770'} 'Hotkey success confirmation did not appear.'
    $messages=Get-Content -LiteralPath (Join-Path $runDirectory 'Logs/capture-diagnostics.log')
    if (-not ($messages | Where-Object {$_ -eq 'notification error=false text=Hotkey settings saved.'})) {throw 'Hotkey success was not confirmed.'}
    Dismiss-Message $dialog 1
    $settings=Get-Content -LiteralPath (Join-Path $runDirectory 'settings.json') -Raw | ConvertFrom-Json
    if (-not $settings.hotkey_enabled -or $settings.hotkey_key -ne 122) {throw 'Hotkey settings did not persist.'}
    [SnapZyTests.Native]::PostMessage($keys,0x10,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
    Wait-For {-not (Window-Of 'SnapZyRustPreview.Hotkeys')} 'Hotkey dialog did not close.' | Out-Null
    [SnapZyTests.Native]::PostMessage($main,0x312,[IntPtr](1),[IntPtr]::Zero) | Out-Null
    $overlay=Wait-For {Window-Of 'SnapZyRustPreview.Selection'} 'Registered hotkey message did not enter capture.'
    [SnapZyTests.Native]::PostMessage($overlay,0x100,[IntPtr](27),[IntPtr]::Zero) | Out-Null
    Wait-For {-not (Window-Of 'SnapZyRustPreview.Selection')} 'Hotkey capture did not cancel.' | Out-Null

    [SnapZyTests.Native]::PostMessage($main,0x800a,[IntPtr](108),[IntPtr]::Zero) | Out-Null
    if (-not $process.WaitForExit(5000)) {throw 'The app did not exit.'}
    if ($process.ExitCode -ne 0) {throw "App exit code: $($process.ExitCode)"}
    $measurements.result='PASS'
    $measurements.coverage='Native launcher EN/TH, Esc, repeat input, region/window/desktop capture, original PNG pixels, Unicode filename, hotkey conflict/success, exit'
    $measurements.limitations='Clipboard DIB serialization is unit-tested; actual clipboard paste and physical key delivery require tester checks. Timings are window detection rather than completed paint.'
    $measurements | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $runDirectory 'results.json') -Encoding utf8
    Write-Output "PASS native desktop smoke checks. Evidence: $runDirectory"
    $measurements | ConvertTo-Json
} finally {
    if ($reserved) {[SnapZyTests.Native]::UnregisterHotKey([IntPtr]::Zero,9881) | Out-Null}
    if ($process -and -not $process.HasExited) {$process.Kill();$process.WaitForExit()}
    if ($fixture) {$fixture.Close();$fixture.Dispose()}
    [SnapZyTests.Native]::SetThreadDpiAwarenessContext($oldDpi) | Out-Null
}
