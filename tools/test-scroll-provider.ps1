param([string]$Executable=(Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-msvc\release\snapzy-rust.exe'))
$ErrorActionPreference='Stop'
Add-Type -AssemblyName PresentationFramework,PresentationCore,WindowsBase,System.Windows.Forms,System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ScrollProviderFixture {
 [StructLayout(LayoutKind.Sequential)] public struct Point {public int X,Y;}
 [StructLayout(LayoutKind.Sequential)] public struct Rect {public int Left,Top,Right,Bottom;}
 [DllImport("user32.dll")] public static extern int GetSystemMetrics(int id);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h,IntPtr after,int x,int y,int w,int height,uint flags);
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
 [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(Point p);
 [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h,uint flags);
}
'@
if([Threading.Thread]::CurrentThread.GetApartmentState() -ne 'STA'){throw 'Run this test on an STA PowerShell thread.'}
$oldDpi=[ScrollProviderFixture]::SetThreadDpiAwarenessContext([IntPtr](-4))
$directory=Join-Path $PSScriptRoot ('..\artifacts\provider-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $directory | Out-Null
$window=[Windows.Window]::new()
$cover=[Windows.Forms.Form]::new()
$child=$null
try {
    $window.Title='SnapZy test-owned WPF scrolling provider'
    $window.Width=500;$window.Height=500;$window.Left=200;$window.Top=100
    $window.ShowInTaskbar=$false;$window.WindowStartupLocation='Manual';$window.ResizeMode='NoResize'
    $viewer=[Windows.Controls.ScrollViewer]::new()
    $viewer.Margin=[Windows.Thickness]::new(24)
    $viewer.VerticalScrollBarVisibility='Auto';$viewer.HorizontalScrollBarVisibility='Disabled'
    $stack=[Windows.Controls.StackPanel]::new()
    for($i=0;$i -lt 80;$i++){
        $row=[Windows.Controls.TextBlock]::new();$row.Text="Provider fixture row $i";$row.Height=32
        $stack.Children.Add($row) | Out-Null
    }
    $viewer.Content=$stack;$window.Content=$viewer
    $window.Show();$window.UpdateLayout();[Windows.Forms.Application]::DoEvents()
    if($viewer.ScrollableHeight -le 0){throw 'WPF fixture has no vertical overflow.'}
    $handle=[Windows.Interop.WindowInteropHelper]::new($window).Handle
    $expected=$viewer.PointToScreen([Windows.Point]::new(0,0))
    $bottom=$viewer.PointToScreen([Windows.Point]::new($viewer.ActualWidth,$viewer.ActualHeight))
    $point=[ScrollProviderFixture+Point]::new();$point.X=[int]$expected.X+100;$point.Y=[int]$expected.Y+100
    $bounds=[ScrollProviderFixture+Rect]::new()
    [ScrollProviderFixture]::GetWindowRect($handle,[ref]$bounds) | Out-Null
    $cover.Text='Test-owned provider covering overlay';$cover.FormBorderStyle='None';$cover.ShowInTaskbar=$false
    [ScrollProviderFixture]::SetWindowPos($cover.Handle,[IntPtr](-1),$bounds.Left,$bounds.Top,$bounds.Right-$bounds.Left,$bounds.Bottom-$bounds.Top,0x50) | Out-Null
    $cover.Update();[Windows.Forms.Application]::DoEvents()
    if([ScrollProviderFixture]::GetAncestor([ScrollProviderFixture]::WindowFromPoint($point),2) -ne $cover.Handle){throw 'Test overlay did not cover the provider hit point.'}
    $request=@{
        point=@($point.X,$point.Y)
        desktop=@([ScrollProviderFixture]::GetSystemMetrics(76),[ScrollProviderFixture]::GetSystemMetrics(77),[ScrollProviderFixture]::GetSystemMetrics(78),[ScrollProviderFixture]::GetSystemMetrics(79))
        windows=@(,@($handle.ToInt64(),@($bounds.Left,$bounds.Top,($bounds.Right-$bounds.Left),($bounds.Bottom-$bounds.Top))))
    }
    $input=Join-Path $directory 'input.json'
    [IO.File]::WriteAllText($input,($request | ConvertTo-Json -Depth 6 -Compress),[Text.UTF8Encoding]::new($false))
    $arguments='--scroll-detect-worker "'+$input+'" --test-data-dir "'+$directory+'"'
    $child=Start-Process -FilePath (Resolve-Path -LiteralPath $Executable) -ArgumentList $arguments -WindowStyle Hidden -PassThru
    $clock=[Diagnostics.Stopwatch]::StartNew()
    while(-not $child.HasExited -and $clock.ElapsedMilliseconds -lt 10000){
        [Windows.Forms.Application]::DoEvents();Start-Sleep -Milliseconds 20;$child.Refresh()
    }
    if(-not $child.HasExited){throw 'Root-based UIA probe timed out.'}
    if($child.ExitCode -ne 0){throw "Root-based UIA helper failed: $($child.ExitCode)"}
    $result=Get-Content -LiteralPath (Join-Path $directory 'result.json') -Raw | ConvertFrom-Json
    if(-not $result -or $result[0] -ne $handle.ToInt64()){throw 'UIA probe returned the covering overlay or no provider.'}
    $rect=$result[1]
    $expectedRect=@([int]$expected.X,[int]$expected.Y,[int]($bottom.X-$expected.X),[int]($bottom.Y-$expected.Y))
    for($i=0;$i -lt 4;$i++){if([Math]::Abs($rect[$i]-$expectedRect[$i]) -gt 3){throw "UIA selected the wrong viewport: actual=$rect expected=$expectedRect"}}
    if($viewer.VerticalOffset -ne 0){throw 'UIA detection moved the source before confirmation.'}
    [pscustomobject]@{Result='PASS';Version=(Get-Item -LiteralPath $Executable).VersionInfo.ProductVersion;Viewport=$rect;OverlayCoveredHitPoint=$true;SourceScrollUnchanged=$true;ElapsedMs=[Math]::Round($clock.Elapsed.TotalMilliseconds,2);Evidence=$directory} | ConvertTo-Json
} finally {
    if($child){if(-not $child.HasExited){$child.Kill();$child.WaitForExit()};$child.Dispose()}
    $cover.Close();$cover.Dispose();$window.Close()
    [ScrollProviderFixture]::SetThreadDpiAwarenessContext($oldDpi) | Out-Null
}
