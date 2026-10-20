param([string]$Executable = (Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-msvc\release\snapzy-rust.exe'),[switch]$EditorChecks,[switch]$PaintChecks,[switch]$AnnotationChecks,[switch]$ResizeChecks,[switch]$ScrollChecks,[switch]$InputProbe,[switch]$CalloutChecks,[switch]$AutoScrollChecks,[switch]$LayerChecks,[switch]$MultiImageChecks,[switch]$OcrChecks,[switch]$MessageOnly,[switch]$DetectStallChecks,[switch]$EscapeInputChecks,[switch]$HighlighterChecks,[switch]$GeometryChecks,[switch]$ShapeResizeChecks,[switch]$SkipShapeResize,[switch]$EffectsChecks,[switch]$ProjectChecks,[ValidateRange(0,3600)][int]$MaxDurationSeconds=0)
$ErrorActionPreference='Stop'
if($SkipShapeResize -and -not $ShapeResizeChecks){throw 'SkipShapeResize requires ShapeResizeChecks.'}
$suiteClock=[Diagnostics.Stopwatch]::StartNew()
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
  [StructLayout(LayoutKind.Sequential)] public struct ScrollInfo {public uint Size,Mask;public int Min,Max;public uint Page;public int Pos,TrackPos;}
  [StructLayout(LayoutKind.Sequential)] public struct MouseInput {public int X,Y;public uint Data,Flags,Time;public UIntPtr Extra;}
  [StructLayout(LayoutKind.Sequential)] public struct Input {public uint Type;public MouseInput Mouse;}
  [StructLayout(LayoutKind.Sequential)] public struct GuiThreadInfo {public uint Size,Flags;public IntPtr Active,Focus,Capture,MenuOwner,MoveSize,Caret;public Rect CaretBounds;}
  [StructLayout(LayoutKind.Sequential)] public struct MenuBarInfo {public uint Size;public Rect Bounds;public IntPtr Menu,Window;public uint Focus;}
  [DllImport("user32.dll")] public static extern bool GetMenuBarInfo(IntPtr h,int obj,int item,ref MenuBarInfo info);
  [DllImport("user32.dll")] public static extern uint GetMenuState(IntPtr menu,uint item,uint flags);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc callback,IntPtr p);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent,EnumProc callback,IntPtr p);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
  [DllImport("user32.dll")] public static extern bool GetGUIThreadInfo(uint thread,ref GuiThreadInfo info);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(Point point);
  [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr window,uint flags);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out Point point);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
  [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int key);
  [DllImport("user32.dll")] public static extern void keybd_event(byte key,byte scan,uint flags,UIntPtr extra);
  public static void EscapeAt(IntPtr source,int pid) {
   uint owner;GetWindowThreadProcessId(source,out owner);
   if(owner!=(uint)pid || GetForegroundWindow()!=source)throw new InvalidOperationException("Escape input requires the test-owned source in foreground.");
   foreach(int key in new[]{1,2,4,16,17,18,27,91,92})if(GetAsyncKeyState(key)<0)throw new InvalidOperationException("Release keys before the Escape input test.");
   try{keybd_event(27,0,0,UIntPtr.Zero);System.Threading.Thread.Sleep(160);}
   finally{keybd_event(27,0,2,UIntPtr.Zero);}
  }
  [DllImport("user32.dll",SetLastError=true)] public static extern uint SendInput(uint count,Input[] input,int size);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h,StringBuilder b,int n);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h,StringBuilder b,int n);
  [DllImport("user32.dll",CharSet=CharSet.Unicode,EntryPoint="SendMessageW")] public static extern IntPtr ReadText(IntPtr h,uint m,IntPtr n,StringBuilder b);
  [DllImport("user32.dll",CharSet=CharSet.Unicode,EntryPoint="SendMessageW")] public static extern IntPtr WriteText(IntPtr h,uint m,IntPtr n,string text);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern bool SetWindowText(IntPtr h,string text);
  [DllImport("user32.dll",CharSet=CharSet.Unicode,EntryPoint="FindWindowW")] public static extern IntPtr FindWindow(string cls,string title);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h,uint m,IntPtr w,IntPtr l);
  [DllImport("user32.dll")] public static extern IntPtr SendMessageTimeout(IntPtr h,uint m,IntPtr w,IntPtr l,uint flags,uint timeout,out IntPtr result);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h,uint m,IntPtr w,IntPtr l);
  [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr h,int id);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h,out Rect r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h,out Rect r);
  [DllImport("user32.dll")] public static extern int SetScrollInfo(IntPtr h,int bar,ref ScrollInfo info,bool redraw);
  [DllImport("user32.dll")] public static extern bool GetScrollInfo(IntPtr h,int bar,ref ScrollInfo info);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h,ref Point p);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h,IntPtr after,int x,int y,int w,int height,uint flags);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h,int command);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
  [DllImport("user32.dll")] public static extern bool UpdateWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetUpdateRect(IntPtr h,out Rect r,bool erase);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
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
  [DllImport("user32.dll")] public static extern bool OpenClipboard(IntPtr h);
  [DllImport("user32.dll")] public static extern bool CloseClipboard();
  [DllImport("user32.dll")] public static extern IntPtr GetClipboardData(uint format);
  [DllImport("user32.dll")] public static extern uint GetGuiResources(IntPtr process,uint flags);
  public static string Class(IntPtr h) {var b=new StringBuilder(128);GetClassName(h,b,b.Capacity);return b.ToString();}
  public static int TestAppPid;
  static bool moved;static Point previous,last;
  public static void InputAt(IntPtr root,int pid,Point point,bool click,int wheel) {
   uint actual;GetWindowThreadProcessId(root,out actual);
   if(actual!=(uint)pid || (pid!=TestAppPid && pid!=System.Diagnostics.Process.GetCurrentProcess().Id))throw new InvalidOperationException("Input target is not a test-owned process.");
   if(GetAncestor(WindowFromPoint(point),2)!=root)throw new InvalidOperationException("Input target is covered; refusing desktop input.");
   foreach(int key in new[]{1,2,4,16,17,18})if((GetAsyncKeyState(key)&0x8000)!=0)throw new InvalidOperationException("User input is held; refusing test input.");
   if(!moved){if(!GetCursorPos(out previous))throw new InvalidOperationException("Cannot preserve cursor.");moved=true;}
   if(!SetCursorPos(point.X,point.Y))throw new InvalidOperationException("Cannot position test cursor.");last=point;
   if(GetAncestor(WindowFromPoint(point),2)!=root)throw new InvalidOperationException("Input target changed; refusing desktop input.");
   Input[] input=click?new[]{new Input{Mouse=new MouseInput{Flags=2}},new Input{Mouse=new MouseInput{Flags=4}}}:new[]{new Input{Mouse=new MouseInput{Flags=0x800,Data=unchecked((uint)wheel)}}};
   uint sent=SendInput((uint)input.Length,input,Marshal.SizeOf(typeof(Input)));
   if(sent!=(uint)input.Length){if(click && sent==1)SendInput(1,new[]{input[1]},Marshal.SizeOf(typeof(Input)));throw new InvalidOperationException("Test input injection was blocked.");}
  }
  public static void PressEscape() { keybd_event(0x1b,0,0,UIntPtr.Zero); keybd_event(0x1b,0,2,UIntPtr.Zero); }
  public static void RestoreCursor() {Point current;if(moved && GetCursorPos(out current) && current.X==last.X && current.Y==last.Y)SetCursorPos(previous.X,previous.Y);}
  public static IntPtr MouseCapture(IntPtr h) {uint pid;uint thread=GetWindowThreadProcessId(h,out pid);var info=new GuiThreadInfo();info.Size=(uint)Marshal.SizeOf(typeof(GuiThreadInfo));if(!GetGUIThreadInfo(thread,ref info))throw new InvalidOperationException("Cannot inspect app mouse capture.");return info.Capture;}
  public static string Text(IntPtr h) {var b=new StringBuilder(32768);ReadText(h,0x000d,(IntPtr)b.Capacity,b);return b.ToString();}
  public static int HighlightedMenuItem(IntPtr h) {var info=new MenuBarInfo();info.Size=(uint)Marshal.SizeOf(typeof(MenuBarInfo));if(!GetMenuBarInfo(h,-4,0,ref info))return -2;for(uint i=0;i<9;i++){if((GetMenuState(info.Menu,i,0x400)&0x80)!=0)return (int)i;}return -1;}
  public static uint MenuState(IntPtr h,uint index) {var info=new MenuBarInfo();info.Size=(uint)Marshal.SizeOf(typeof(MenuBarInfo));if(!GetMenuBarInfo(h,-4,0,ref info))throw new InvalidOperationException("Cannot inspect test menu.");return GetMenuState(info.Menu,index,0x400);}
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
  public static void ConfigureVerticalScroll(IntPtr h) {var info=new ScrollInfo{Size=(uint)Marshal.SizeOf(typeof(ScrollInfo)),Mask=0x17,Min=0,Max=10000,Page=1,Pos=0};SetScrollInfo(h,1,ref info,false);var check=new ScrollInfo{Size=(uint)Marshal.SizeOf(typeof(ScrollInfo)),Mask=0x17};if(!GetScrollInfo(h,1,ref check)||check.Max<=check.Page)throw new InvalidOperationException("Native scroll range fixture did not initialize.");}
  public static void ClearVerticalScroll(IntPtr h) {var info=new ScrollInfo{Size=(uint)Marshal.SizeOf(typeof(ScrollInfo)),Mask=0x17,Min=0,Max=0,Page=0,Pos=0};SetScrollInfo(h,1,ref info,false);}
 }
}
'@
if ($AutoScrollChecks) {
$gdiplusReference=[AppDomain]::CurrentDomain.GetAssemblies() | Where-Object {$_.GetName().Name -eq 'System.Private.Windows.GdiPlus'} | Select-Object -ExpandProperty Location -First 1
$windowsCoreReference=[AppDomain]::CurrentDomain.GetAssemblies() | Where-Object {$_.GetName().Name -eq 'System.Private.Windows.Core'} | Select-Object -ExpandProperty Location -First 1
$scrollFixtureReferences=@([Drawing.Bitmap].Assembly.Location,[Drawing.Color].Assembly.Location,[Windows.Forms.Form].Assembly.Location,$gdiplusReference,$windowsCoreReference)
Add-Type -ReferencedAssemblies $scrollFixtureReferences @'
using System.Drawing;
namespace SnapZyTests {
 public static class ScrollFixture {
  public static Bitmap Image(int offset,Color background) {
   var b=new Bitmap(240,400);using(var g=Graphics.FromImage(b))g.Clear(background);
   for(int y=0;y<400;y++)for(int x=0;x<240;x++) {
    Color c;
    if(y<55)c=Color.FromArgb(45,130,15);
    else if(y>=358)c=(y==358 || (y>=363 && x<120))?Color.FromArgb(190,190,190):Color.FromArgb(245,245,245);
    else {int row=y-55+offset,cell=row/37+row/83;
     c=(row%37==0 || x%61==0)?Color.FromArgb(30,30,30):
      ((x*19+cell*53)%97<16 && row%37>8 && row%37<27)?Color.FromArgb(50,cell%137,row%127):Color.FromArgb(250,250,250);
    }
    b.SetPixel(x,y,c);
   }
   return b;
  }
 }
}
'@
}
$root=Split-Path $PSScriptRoot
$runDirectory=Join-Path $root ('artifacts\smoke-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $runDirectory -Force | Out-Null
$Executable=(Resolve-Path -LiteralPath $Executable).Path
$appTitle='SnapZy Rust Preview '+(Get-Item -LiteralPath $Executable).VersionInfo.ProductVersion
$process=$null
$reserved=$false
$fixture=$null
$oldDpi=[SnapZyTests.Native]::SetThreadDpiAwarenessContext([IntPtr](-4))
$measurements=[ordered]@{}
$measurements.executable_version=(Get-Item -LiteralPath $Executable).VersionInfo.ProductVersion
$measurements.executable_sha256=(Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
$thaiText=-join (@(0x0e20,0x0e32,0x0e29,0x0e32,0x0e44,0x0e17,0x0e22) | ForEach-Object {[char]$_})
$thaiBlur=-join (@(0x0e40,0x0e1a,0x0e25,0x0e2d) | ForEach-Object {[char]$_})
$thaiFront=-join (@(0x0e19,0x0e33,0x0e02,0x0e36,0x0e49,0x0e19,0x0e1a,0x0e19,0x0e2a,0x0e38,0x0e14) | ForEach-Object {[char]$_})
function Wait-For([scriptblock]$Condition,[string]$Failure,[int]$Timeout=5000) {
    $watch=[Diagnostics.Stopwatch]::StartNew()
    while ($watch.ElapsedMilliseconds -lt $Timeout) {
        if($MaxDurationSeconds -gt 0 -and $suiteClock.Elapsed.TotalSeconds -ge $MaxDurationSeconds) {throw 'Approved test interval expired; remaining checks are unverified.'}
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
    $windows=@([SnapZyTests.Native]::Windows($process.Id,$Class) | Where-Object {$Class -ne '#32770' -or [SnapZyTests.Native]::IsWindowVisible($_)})
    if ($windows.Length) {
        $window=$windows[0]
        if ($Class -eq 'SnapZyRustPreview.Capture' -and [SnapZyTests.Native]::GetDlgItem($window,401) -eq [IntPtr]::Zero) {return $null}
        if ($Class -eq 'SnapZyRustPreview.Hotkeys' -and [SnapZyTests.Native]::GetDlgItem($window,301) -eq [IntPtr]::Zero) {return $null}
        if ($Class -eq 'SnapZyRustPreview.Text' -and (-not [SnapZyTests.Native]::IsWindowVisible($window) -or [SnapZyTests.Native]::GetDlgItem($window,701) -eq [IntPtr]::Zero -or [SnapZyTests.Native]::GetDlgItem($window,1) -eq [IntPtr]::Zero)) {return $null}
        if ($Class -eq 'SnapZyRustPreview.Ocr' -and (-not [SnapZyTests.Native]::IsWindowVisible($window) -or [SnapZyTests.Native]::GetDlgItem($window,904) -eq [IntPtr]::Zero)) {return $null}
        if ($Class -eq '#32770' -and [SnapZyTests.Native]::GetDlgItem($window,1) -eq [IntPtr]::Zero -and [SnapZyTests.Native]::GetDlgItem($window,6) -eq [IntPtr]::Zero -and [SnapZyTests.Native]::Text($window) -ne $appTitle) {return $null}
        return $window
    }
    if ($Class -eq '#32770') {
        $dialog=[SnapZyTests.Native]::FindWindow('#32770',$appTitle)
        if ($dialog -ne [IntPtr]::Zero -and [SnapZyTests.Native]::IsWindowVisible($dialog)) {return $dialog}
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
    $dirty=[SnapZyTests.Native]::Text($Preview).Contains('* ')
    [SnapZyTests.Native]::PostMessage($Preview,0x10,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
    if($dirty){
        $dialog=Wait-For {Window-Of '#32770'} 'Unsaved project confirmation did not appear.'
        Click $dialog 7
    }
    Wait-For {-not [SnapZyTests.Native]::IsWindow($Preview)} 'Preview did not close.' | Out-Null
}
function Mouse([IntPtr]$Overlay,[uint32]$Message,[int]$X,[int]$Y) {
    $parameter=([int64]($Y-band 0xffff)-shl 16)-bor ($X-band 0xffff)
    [SnapZyTests.Native]::PostMessage($Overlay,$Message,[IntPtr](1),[IntPtr]$parameter) | Out-Null
}
function Select-TestShape([IntPtr]$Preview,[int]$Tool) {
    $button=if($Tool -in @(637,638)){637}else{606}
    $index=if($Tool -in @(636,638)){1}else{0}
    Click $Preview $button
    $menu=Wait-For {[SnapZyTests.Native]::Windows($process.Id,'#32768') | Where-Object {[SnapZyTests.Native]::IsWindowVisible($_)} | Select-Object -First 1} 'Shape popup missing.'
    for($i=0;$i -lt 4 -and [SnapZyTests.Native]::HighlightedMenuItem($menu) -ne $index;$i++) {
        $old=[SnapZyTests.Native]::HighlightedMenuItem($menu)
        [SnapZyTests.Native]::PostMessage($Preview,0x100,[IntPtr](40),[IntPtr]::Zero) | Out-Null
        Wait-For {$state=[SnapZyTests.Native]::HighlightedMenuItem($menu);$state -ge 0 -and $state -ne $old} 'Shape popup did not advance.' | Out-Null
    }
    if([SnapZyTests.Native]::HighlightedMenuItem($menu) -ne $index){throw 'Shape popup selected wrong item.'}
    [SnapZyTests.Native]::PostMessage($Preview,0x100,[IntPtr](13),[IntPtr]::Zero) | Out-Null
    Wait-For {-not [SnapZyTests.Native]::IsWindowVisible($menu)} 'Shape popup did not close.' | Out-Null
    Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($Preview,$button)) -match $(if($index -eq 1){'ellipse|วงรี'}else{'rectangle|สี่เหลี่ยม'})} 'Shape accessible label did not update.' | Out-Null
}
function Save-Png([IntPtr]$Preview,[string]$Name) {
    # A posted appearance-only button message is an acknowledgement from the
    # same UI queue as prior gestures. Checking IsWindowEnabled alone can see
    # the old enabled state while a font layout command is still executing.
    $save=[SnapZyTests.Native]::GetDlgItem($Preview,202)
    [SnapZyTests.Native]::SendMessage($save,0xf3,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
    [SnapZyTests.Native]::PostMessage($save,0xf3,[IntPtr](1),[IntPtr]::Zero) | Out-Null
    Wait-For {([SnapZyTests.Native]::SendMessage($save,0xf2,[IntPtr]::Zero,[IntPtr]::Zero).ToInt64()-band 4) -ne 0} 'Editor did not acknowledge pending gestures.' 15000 | Out-Null
    [SnapZyTests.Native]::SendMessage($save,0xf3,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
    Wait-For {[SnapZyTests.Native]::IsWindowEnabled([SnapZyTests.Native]::GetDlgItem($Preview,202))} 'Composition did not become exportable.' 15000 | Out-Null
    Click $Preview 202
    $dialog=Wait-For {Window-Of '#32770'} 'PNG dialog did not open.'
    $filename=Wait-For {
        foreach ($c in [SnapZyTests.Native]::Children($dialog)) {
            if ([SnapZyTests.Native]::Class($c) -eq 'Edit' -and [SnapZyTests.Native]::Text($c).Contains('SnapZy-Rust-')) {return $c}
        };return $null
    } 'Save filename missing.'
    $path=Join-Path $runDirectory $Name
    [SnapZyTests.Native]::WriteText($filename,0x000c,[IntPtr]::Zero,$path) | Out-Null
    Click $dialog 1
    try {Wait-For {(Test-Path -LiteralPath $path) -and [SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($Preview,401)) -eq 'Saved'} 'PNG export failed.' | Out-Null}
    catch {throw "PNG export failed: $path; exists=$(Test-Path -LiteralPath $path); status=$([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($Preview,401))); windows=$([SnapZyTests.Native]::ProcessWindows($process.Id))"}
    return $path
}
function Import-TestImage([IntPtr]$Preview,[string]$Path,[switch]$ExpectError) {
    Click $Preview 629
    $dialog=Wait-For {[SnapZyTests.Native]::Windows($process.Id,'#32770') | Where-Object {[SnapZyTests.Native]::IsWindowVisible($_) -and [SnapZyTests.Native]::GetDlgItem($_,1148) -ne [IntPtr]::Zero} | Select-Object -First 1} 'Image browse dialog missing.'
    $combo=[SnapZyTests.Native]::GetDlgItem($dialog,1148)
    $filename=[SnapZyTests.Native]::Children($combo) | Where-Object {[SnapZyTests.Native]::Class($_) -eq 'Edit'} | Select-Object -First 1
    if(-not $filename){$filename=[SnapZyTests.Native]::GetDlgItem($dialog,1152)}
    if(-not $filename){throw 'Image filename field missing.'}
    [SnapZyTests.Native]::WriteText($filename,0xc,[IntPtr]::Zero,$Path) | Out-Null
    if([SnapZyTests.Native]::Text($filename) -ne $Path){throw 'Import filename was not applied.'}
    Click $dialog 1
    Wait-For {-not [SnapZyTests.Native]::IsWindowVisible($dialog)} 'Image browse did not close.' | Out-Null
    if($ExpectError) {
        $errorDialog=Wait-For {
            $candidate=Window-Of '#32770'
            if($candidate) {
                $message=([SnapZyTests.Native]::Children($candidate) | ForEach-Object {[SnapZyTests.Native]::Text($_)}) -join ' '
                if($message -match 'Image memory limit reached|Choose a PNG, JPEG or BMP image') {return $candidate}
            }
            return $null
        } 'Invalid/oversized image did not report an error.'
        Dismiss-Message $errorDialog 1
        return
    }
    try {Wait-For {([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($Preview,401)) -match '^\d+ x \d+ px$') -and [SnapZyTests.Native]::IsWindowEnabled([SnapZyTests.Native]::GetDlgItem($Preview,202))} 'Image import did not finish composing.' | Out-Null}
    catch {throw "Image import did not finish: $Path; status=$([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($Preview,401))); windows=$([SnapZyTests.Native]::ProcessWindows($process.Id))"}
}
function Image-View([IntPtr]$Preview,[int]$Width,[int]$Height) {
    $r=[SnapZyTests.Native]::Client($Preview)
    return @([int](($r.Right-$r.Left-$Width)/2),(145+[int](($r.Bottom-$r.Top-175-$Height)/2)))
}
function Set-Parameter([IntPtr]$Preview,[int]$Value) {
    # Drain posted selection gestures before directly writing a numeric control.
    Save-Png $Preview ('parameter-before-'+[guid]::NewGuid().ToString('N')+'.png') | Out-Null
    $field=[SnapZyTests.Native]::GetDlgItem($Preview,611)
    [SnapZyTests.Native]::WriteText($field,0x000c,[IntPtr]::Zero,$Value.ToString()) | Out-Null
    [SnapZyTests.Native]::PostMessage($Preview,0x111,[IntPtr](611-bor (512-shl 16)),$field) | Out-Null
}
function Set-LayerTestColor([IntPtr]$Preview,[int]$Index) {
    Click $Preview 610
    $menu=Wait-For {[SnapZyTests.Native]::Windows($process.Id,'#32768') | Where-Object {[SnapZyTests.Native]::IsWindowVisible($_)} | Select-Object -First 1} 'Layer color menu did not open.'
    for($i=0;$i -lt 12 -and [SnapZyTests.Native]::HighlightedMenuItem($menu) -ne $Index;$i++) {
        $old=[SnapZyTests.Native]::HighlightedMenuItem($menu)
        [SnapZyTests.Native]::PostMessage($Preview,0x100,[IntPtr](40),[IntPtr]::Zero) | Out-Null
        Wait-For {$state=[SnapZyTests.Native]::HighlightedMenuItem($menu);$state -ge 0 -and $state -ne $old} 'Layer color menu did not advance.' | Out-Null
    }
    if([SnapZyTests.Native]::HighlightedMenuItem($menu) -ne $Index){throw 'Layer color was not selected.'}
    [SnapZyTests.Native]::PostMessage($Preview,0x100,[IntPtr](13),[IntPtr]::Zero) | Out-Null
    Wait-For {-not [SnapZyTests.Native]::IsWindowVisible($menu)} 'Layer color menu did not close.' | Out-Null
}
function Assert-LayerPixel([string]$Path,[Drawing.Color]$Color) {
    $image=[Drawing.Bitmap]::FromFile($Path)
    try {
        if($image.Width -ne 160 -or $image.Height -ne 80){throw 'Layer export changed native dimensions.'}
        if($image.GetPixel(60,30).ToArgb() -ne $Color.ToArgb()){throw "Layer overlap pixel is wrong in $Path."}
    }finally{$image.Dispose()}
}
function Snapshot-Editor([IntPtr]$Preview,[string]$Name,[switch]$KeepPosition) {
    if(-not $KeepPosition){[SnapZyTests.Native]::SetWindowPos($Preview,[IntPtr](-1),$screen.Left+20,$screen.Top+200,0,0,1) | Out-Null}
    [SnapZyTests.Native]::UpdateWindow($Preview) | Out-Null
    [SnapZyTests.Native]::DwmFlush() | Out-Null
    $r=[SnapZyTests.Native]::Client($Preview)
    $bitmap=[Drawing.Bitmap]::new($r.Right-$r.Left,$r.Bottom-$r.Top)
    $graphics=[Drawing.Graphics]::FromImage($bitmap)
    $target=$graphics.GetHdc();$source=[SnapZyTests.Native]::GetDC($Preview)
    try {
        if (-not [SnapZyTests.Native]::BitBlt($target,0,0,$bitmap.Width,$bitmap.Height,$source,0,0,0x00CC0020)) {throw 'Editor screenshot failed.'}
        [SnapZyTests.Native]::GdiFlush() | Out-Null
    } finally {[SnapZyTests.Native]::ReleaseDC($Preview,$source) | Out-Null;$graphics.ReleaseHdc($target);$graphics.Dispose()}
    try {$bitmap.Save((Join-Path $runDirectory $Name),[Drawing.Imaging.ImageFormat]::Png)}finally{$bitmap.Dispose()}
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
    $fixture.Location=[Drawing.Point]::new($screen.Right-330,$screen.Top+100)
    $fixture.Show();[Windows.Forms.Application]::DoEvents()
    $clock=[Diagnostics.Stopwatch]::StartNew()
    $previousDetectionDelay=$env:SNAPZY_TEST_UIA_HANDOFF_DELAY_MS
    $previousDetectionStall=$env:SNAPZY_TEST_UIA_STALL_MS
    if($ScrollChecks) {$env:SNAPZY_TEST_UIA_HANDOFF_DELAY_MS='350'}
    if($DetectStallChecks) {$env:SNAPZY_TEST_UIA_STALL_MS='12000'}
    try {$process=Start-Process -FilePath $Executable -ArgumentList @('--test-data-dir',"`"$runDirectory`"") -WindowStyle Hidden -RedirectStandardError (Join-Path $runDirectory 'app-stderr.log') -PassThru}
    finally {$env:SNAPZY_TEST_UIA_HANDOFF_DELAY_MS=$previousDetectionDelay;$env:SNAPZY_TEST_UIA_STALL_MS=$previousDetectionStall}
    [SnapZyTests.Native]::TestAppPid=$process.Id
    $main=Wait-For {Window-Of 'SnapZyRustPreview.Launcher'} 'Launcher did not initialize.'
    if ($AutoScrollChecks) {
        if ([SnapZyTests.Native]::GetDlgItem($main,110) -eq [IntPtr]::Zero -or [SnapZyTests.Native]::GetDlgItem($main,111) -eq [IntPtr]::Zero) {throw 'Automatic/Manual scrolling choice is missing.'}
        if ([SnapZyTests.Native]::SendMessage([SnapZyTests.Native]::GetDlgItem($main,110),0xf0,[IntPtr]::Zero,[IntPtr]::Zero).ToInt64() -ne 1) {throw 'Automatic scrolling is not the default.'}
    }
    Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($main,401)) -eq 'Ready'} 'Launcher controls did not become ready.' | Out-Null
    Wait-For {
        foreach($log in Get-ChildItem (Join-Path $runDirectory 'Logs') -Filter 'performance-*.log' -ErrorAction SilentlyContinue) {
            if (Get-Content -LiteralPath $log.FullName | Where-Object {$_ -match ' launcher.ready '}) {return $true}
        }
        return $false
    } 'Launcher ready timing was not logged.' | Out-Null
    $measurements.launcher_window_ms=[Math]::Round($clock.Elapsed.TotalMilliseconds,2)
    # Start-Process Hidden can override the app's first ShowWindow call. Make
    # the test-owned launcher visible before checking rendered UI screenshots.
    [SnapZyTests.Native]::ShowWindow($main,9) | Out-Null
    Wait-For {[SnapZyTests.Native]::IsWindowVisible($main)} 'Test launcher did not become visible.' | Out-Null
    [SnapZyTests.Native]::SetWindowPos($main,[IntPtr](-1),$screen.Left+20,$screen.Top+20,0,0,1) | Out-Null
    if ([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($main,104)) -ne 'EN') {throw 'English default was not applied.'}
    if ($AutoScrollChecks) {Snapshot-Editor $main 'launcher-auto-en.png' -KeepPosition}
    Click $main 104
    Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($main,104)) -eq 'TH'} 'Language did not switch to Thai.' | Out-Null
    if ($AutoScrollChecks) {Snapshot-Editor $main 'launcher-auto-th.png' -KeepPosition}
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
    if ($ProjectChecks) {
        function Select-ProjectSave([IntPtr]$Owner,[switch]$As) {
            Click $Owner 642
            $menu=Wait-For {[SnapZyTests.Native]::Windows($process.Id,'#32768') | Where-Object {[SnapZyTests.Native]::IsWindowVisible($_)} | Select-Object -First 1} 'Project save menu missing.'
            $index=if($As){1}else{0}
            for($i=0;$i -lt 4 -and [SnapZyTests.Native]::HighlightedMenuItem($menu) -ne $index;$i++) {
                [SnapZyTests.Native]::PostMessage($Owner,0x100,[IntPtr](40),[IntPtr]::Zero) | Out-Null
                Start-Sleep -Milliseconds 60
            }
            if([SnapZyTests.Native]::HighlightedMenuItem($menu) -ne $index){throw 'Wrong project save menu item.'}
            [SnapZyTests.Native]::PostMessage($Owner,0x100,[IntPtr](13),[IntPtr]::Zero) | Out-Null
            Wait-For {-not [SnapZyTests.Native]::IsWindowVisible($menu)} 'Project save menu did not close.' | Out-Null
        }
        function Project-FileDialog([string]$Path) {
            $match=Wait-For {
                foreach($candidate in [SnapZyTests.Native]::Windows($process.Id,'#32770')) {
                    $field=[SnapZyTests.Native]::Children($candidate) | Where-Object {[SnapZyTests.Native]::Class($_) -eq 'Edit' -and [SnapZyTests.Native]::Text($_).EndsWith('.snapzy')} | Select-Object -First 1
                    if($field){return @{Dialog=$candidate;Edit=$field}}
                }
            } 'Project filename field missing.'
            $dialog=$match.Dialog;$edit=$match.Edit
            [SnapZyTests.Native]::WriteText($edit,0xc,[IntPtr]::Zero,$Path) | Out-Null
            if([SnapZyTests.Native]::Text($edit) -ne $Path){throw 'Project path was not applied.'}
            Click $dialog 1
            return $dialog
        }
        function Wait-ProjectSaved([IntPtr]$Owner) {
            Wait-For { -not [SnapZyTests.Native]::Text($Owner).Contains('* ') -and [SnapZyTests.Native]::IsWindowEnabled([SnapZyTests.Native]::GetDlgItem($Owner,642))} 'Project did not finish saving.' 15000 | Out-Null
        }
        $v=Image-View $preview 160 80
        Click $preview 604
        Mouse $preview 0x201 ($v[0]+5) ($v[1]+8)
        Mouse $preview 0x202 ($v[0]+70) ($v[1]+8)
        Click $preview 617
        Mouse $preview 0x201 ($v[0]+10) ($v[1]+30)
        $entry=Wait-For {Window-Of 'SnapZyRustPreview.Text'} 'Project text fixture dialog missing.'
        [SnapZyTests.Native]::WriteText([SnapZyTests.Native]::GetDlgItem($entry,701),0xc,[IntPtr]::Zero,'ไทย English') | Out-Null
        Click $entry 1
        Wait-For {-not (Window-Of 'SnapZyRustPreview.Text')} 'Project text fixture did not apply.' | Out-Null
        $before=Save-Png $preview 'project-before.png'
        [SnapZyTests.Native]::PostMessage($preview,0x10,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
        $prompt=Wait-For {Window-Of '#32770'} 'PNG export incorrectly suppressed project warning.'
        Click $prompt 2
        Wait-For {-not (Window-Of '#32770')} 'Close Cancel did not dismiss prompt.' | Out-Null
        if(-not [SnapZyTests.Native]::IsWindow($preview)){throw 'Close Cancel discarded work.'}
        Select-ProjectSave $preview
        $warning=Wait-For {Window-Of '#32770'} 'Project privacy warning missing.'
        Click $warning 2
        Wait-For {-not (Window-Of '#32770')} 'Privacy cancellation did not dismiss warning.' | Out-Null
        if(-not [SnapZyTests.Native]::Text($preview).Contains('* ')){throw 'Cancelled save marked work saved.'}
        Select-ProjectSave $preview
        $warning=Wait-For {Window-Of '#32770'} 'Repeated first-save privacy warning missing.'
        Click $warning 1
        $projectPath=Join-Path $runDirectory 'project-งาน.snapzy'
        Project-FileDialog $projectPath | Out-Null
        Wait-ProjectSaved $preview
        if(-not (Test-Path -LiteralPath $projectPath)){throw 'Project file not created.'}
        $windows=@([SnapZyTests.Native]::Windows($process.Id,'SnapZyRustPreview.Capture'))
        Click $main 641
        Project-FileDialog $projectPath | Out-Null
        $loaded=Wait-For {[SnapZyTests.Native]::Windows($process.Id,'SnapZyRustPreview.Capture') | Where-Object {$_ -notin $windows} | Select-Object -First 1} 'Project did not open in a separate editor.'
        Wait-For {[SnapZyTests.Native]::GetDlgItem($loaded,642) -ne [IntPtr]::Zero} 'Loaded editor controls missing.' | Out-Null
        if(-not [SnapZyTests.Native]::IsWindow($preview)){throw 'Open replaced original editor.'}
        if([SnapZyTests.Native]::IsWindowEnabled([SnapZyTests.Native]::GetDlgItem($loaded,607)) -or [SnapZyTests.Native]::IsWindowEnabled([SnapZyTests.Native]::GetDlgItem($loaded,608))){throw 'Reopened project retained history.'}
        $after=Save-Png $loaded 'project-reopened.png'
        if((Get-FileHash $before).Hash -ne (Get-FileHash $after).Hash){throw 'Project round-trip changed native PNG pixels.'}
        $v2=Image-View $loaded 160 80
        Click $loaded 601
        Mouse $loaded 0x203 ($v2[0]+14) ($v2[1]+34)
        $entry=Wait-For {Window-Of 'SnapZyRustPreview.Text'} 'Reopened text is not editable.'
        if([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($entry,701)) -ne 'ไทย English'){throw 'Reopened text changed.'}
        [SnapZyTests.Native]::WriteText([SnapZyTests.Native]::GetDlgItem($entry,701),0xc,[IntPtr]::Zero,'แก้ไข English') | Out-Null
        Click $entry 1
        Wait-For {[SnapZyTests.Native]::Text($loaded).Contains('* ')} 'Loaded edit did not mark project dirty.' | Out-Null
        Click $loaded 607
        Wait-For {-not [SnapZyTests.Native]::Text($loaded).Contains('* ')} 'Undo-to-saved stayed dirty.' | Out-Null
        Click $loaded 608
        Wait-For {[SnapZyTests.Native]::Text($loaded).Contains('* ')} 'Redo-away-from-saved stayed clean.' | Out-Null
        Click $preview 604
        Mouse $preview 0x201 ($v[0]+5) ($v[1]+70)
        Mouse $preview 0x202 ($v[0]+70) ($v[1]+70)
        Wait-For {[SnapZyTests.Native]::Text($preview).Contains('* ')} 'Original editor edit did not become dirty.' | Out-Null
        Click $main 108
        $prompt=Wait-For {Window-Of '#32770'} 'Exit did not ask about first project.'
        Click $prompt 7
        Wait-For {-not [SnapZyTests.Native]::IsWindow($prompt)} 'First exit prompt did not close.' | Out-Null
        $prompt=Wait-For {Window-Of '#32770'} 'Exit did not ask about second project.'
        Click $prompt 2
        Wait-For {-not (Window-Of '#32770')} 'Exit Cancel did not complete.' | Out-Null
        if(-not [SnapZyTests.Native]::IsWindow($preview) -or -not [SnapZyTests.Native]::IsWindow($loaded)){throw 'Exit Cancel prematurely discarded an editor.'}
        Click $loaded 607
        Click $preview 607
        Wait-ProjectSaved $loaded
        Wait-ProjectSaved $preview
        [SnapZyTests.Native]::PostMessage($loaded,0x10,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
        Wait-For {-not [SnapZyTests.Native]::IsWindow($loaded)} 'Clean project did not close.' | Out-Null
        Select-ProjectSave $preview -As
        $warning=Wait-For {Window-Of '#32770'} 'Save As privacy warning missing.'
        Click $warning 1
        $secondPath=Join-Path $runDirectory 'project-copy.snapzy'
        Project-FileDialog $secondPath | Out-Null
        Wait-ProjectSaved $preview
        if(-not [SnapZyTests.Native]::Text($preview).Contains('project-copy.snapzy')){throw 'Save As did not associate the new path.'}
        Click $preview 604
        Mouse $preview 0x201 ($v[0]+6) ($v[1]+71)
        Mouse $preview 0x202 ($v[0]+71) ($v[1]+71)
        [SnapZyTests.Native]::PostMessage($preview,0x10,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
        $prompt=Wait-For {Window-Of '#32770'} 'Close Save prompt missing.'
        Click $prompt 6
        Wait-For {-not [SnapZyTests.Native]::IsWindow($preview)} 'Close Save did not finish and close.' 15000 | Out-Null
        Click $main 641
        Project-FileDialog $secondPath | Out-Null
        $preview=Wait-For {Window-Of 'SnapZyRustPreview.Capture'} 'Saved-on-close project could not reopen.'
        Snapshot-Editor $preview 'project-reopened-editor.png'
        $measurements.projects='PASS: first-save privacy cancel/accept, Unicode path, native exact PNG round-trip, new editor, reset history, text edit, Undo/Redo saved marker, staged multi-editor Exit Cancel, Save As association, Save-on-close and reopen'
    }
    if ($EffectsChecks) {
        function Open-Effects {
            Click $preview 640
            $panel=Wait-For {Window-Of 'SnapZyRustPreview.Effects'} 'Effects panel missing.'
            Wait-For {[SnapZyTests.Native]::GetDlgItem($panel,1107) -ne [IntPtr]::Zero -and [SnapZyTests.Native]::IsWindowVisible($panel)} 'Effects controls not ready.' | Out-Null
            return $panel
        }
        function Set-EffectField([IntPtr]$Panel,[int]$Id,[string]$Value) {
            [SnapZyTests.Native]::WriteText([SnapZyTests.Native]::GetDlgItem($Panel,$Id),0xc,[IntPtr]::Zero,$Value) | Out-Null
        }
        $v=Image-View $preview 160 80
        Select-TestShape $preview 637
        Mouse $preview 0x201 ($v[0]+20) ($v[1]+20)
        Mouse $preview 0x202 ($v[0]+100) ($v[1]+60)
        $plain=Save-Png $preview 'effects-plain.png'
        $panel=Open-Effects
        Snapshot-Editor $panel 'effects-shape-en.png' -KeepPosition
        foreach($pair in @(@(1102,'6'),@(1103,'3'),@(1104,'3'),@(1107,'2'))) {
            if([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($panel,$pair[0])) -ne $pair[1]){throw 'Effect defaults differ.'}
        }
        if([SnapZyTests.Native]::IsWindowEnabled([SnapZyTests.Native]::GetDlgItem($panel,1105))){throw 'Outline is enabled for a shape.'}
        [SnapZyTests.Native]::SendMessage([SnapZyTests.Native]::GetDlgItem($panel,1100),0xf1,[IntPtr](1),[IntPtr]::Zero) | Out-Null
        foreach($pair in @(@(1102,'0'),@(1103,'8'),@(1104,'8'))){Set-EffectField $panel $pair[0] $pair[1]}
        Click $panel 1
        Wait-For {-not [SnapZyTests.Native]::IsWindow($panel)} 'Effects Apply did not close.' | Out-Null
        $shadow=Save-Png $preview 'effects-shadow.png'
        $b=[Drawing.Bitmap]::FromFile($shadow)
        try {if($b.GetPixel(105,65).ToArgb() -ne [Drawing.Color]::Black.ToArgb() -or $b.GetPixel(30,30).ToArgb() -ne [Drawing.Color]::Red.ToArgb()){throw 'Shadow or object pixel differs.'}}finally{$b.Dispose()}
        Click $preview 607
        if((Get-FileHash (Save-Png $preview 'effects-undo.png')).Hash -ne (Get-FileHash $plain).Hash){throw 'Effects are not one Undo step.'}
        Click $preview 608
        if((Get-FileHash (Save-Png $preview 'effects-redo.png')).Hash -ne (Get-FileHash $shadow).Hash){throw 'Effect Redo differs.'}
        $panel=Open-Effects
        Set-EffectField $panel 1103 '-8'
        Click $panel 2
        Wait-For {-not [SnapZyTests.Native]::IsWindow($panel)} 'Effects Cancel did not close.' | Out-Null
        if((Get-FileHash (Save-Png $preview 'effects-cancel.png')).Hash -ne (Get-FileHash $shadow).Hash){throw 'Effects Cancel changed pixels.'}
        $panel=Open-Effects
        Set-EffectField $panel 1102 '32'
        [SnapZyTests.Native]::PostMessage($panel,0x100,[IntPtr](27),[IntPtr]::Zero) | Out-Null
        Wait-For {-not [SnapZyTests.Native]::IsWindow($panel)} 'Effects Escape did not close.' | Out-Null
        if((Get-FileHash (Save-Png $preview 'effects-escape.png')).Hash -ne (Get-FileHash $shadow).Hash){throw 'Effects Escape changed pixels.'}
        Click $preview 201
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -eq 'Copied'} 'Effects copy failed.' | Out-Null
        $a=[Drawing.Bitmap]::FromFile($shadow);$copy=[Windows.Forms.Clipboard]::GetImage();$b=[Drawing.Bitmap]::new($copy)
        try {for($y=0;$y -lt 80;$y++){for($x=0;$x -lt 160;$x++){if($a.GetPixel($x,$y).ToArgb() -ne $b.GetPixel($x,$y).ToArgb()){throw 'Effects clipboard differs from PNG.'}}}}finally{$a.Dispose();$b.Dispose();$copy.Dispose()}
        Click $preview 609
        Save-Png $preview 'effects-empty.png' | Out-Null
        Click $preview 617
        Mouse $preview 0x201 ($v[0]+20) ($v[1]+10)
        $entry=Wait-For {Window-Of 'SnapZyRustPreview.Text'} 'Effects text entry missing.'
        [SnapZyTests.Native]::WriteText([SnapZyTests.Native]::GetDlgItem($entry,701),0xc,[IntPtr]::Zero,"$thaiText A") | Out-Null
        Click $entry 1
        Wait-For {-not [SnapZyTests.Native]::IsWindowVisible($entry)} 'Effects text entry did not close.' | Out-Null
        $text=Save-Png $preview 'effects-text.png'
        $panel=Open-Effects
        if(-not [SnapZyTests.Native]::IsWindowEnabled([SnapZyTests.Native]::GetDlgItem($panel,1105))){throw 'Text outline control disabled.'}
        [SnapZyTests.Native]::SendMessage([SnapZyTests.Native]::GetDlgItem($panel,1105),0xf1,[IntPtr](1),[IntPtr]::Zero) | Out-Null
        Set-EffectField $panel 1107 '2'
        Snapshot-Editor $panel 'effects-text-en.png' -KeepPosition
        Click $panel 1
        Wait-For {-not [SnapZyTests.Native]::IsWindow($panel)} 'Outline Apply did not close.' | Out-Null
        $outlined=Save-Png $preview 'effects-text-outline.png'
        if((Get-FileHash $outlined).Hash -eq (Get-FileHash $text).Hash){throw 'Outline produced no pixels.'}
        Click $preview 614
        $panel=Open-Effects
        Snapshot-Editor $panel 'effects-text-th.png' -KeepPosition
        Set-EffectField $panel 1107 '33'
        Click $panel 1
        $warning=Wait-For {[SnapZyTests.Native]::Windows($process.Id,'#32770') | Where-Object {[SnapZyTests.Native]::IsWindowVisible($_)} | Select-Object -First 1} 'Invalid effect settings were accepted.'
        Dismiss-Message $warning 1
        if(-not [SnapZyTests.Native]::IsWindow($panel)){throw 'Invalid settings closed panel.'}
        Click $panel 2
        Wait-For {-not [SnapZyTests.Native]::IsWindow($panel)} 'Effects TH cancel did not close.' | Out-Null
        Click $preview 614
        Click $preview 609
        Save-Png $preview 'effects-text-deleted.png' | Out-Null
        $panel=Open-Effects
        [SnapZyTests.Native]::SendMessage([SnapZyTests.Native]::GetDlgItem($panel,1100),0xf1,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
        [SnapZyTests.Native]::SendMessage([SnapZyTests.Native]::GetDlgItem($panel,1105),0xf1,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
        Click $panel 1
        Wait-For {-not [SnapZyTests.Native]::IsWindow($panel)} 'Effects reset did not close.' | Out-Null
        Click $preview 603
        Save-Png $preview 'effects-reset.png' | Out-Null
        $measurements.effects='PASS'
    }
    if ($ShapeResizeChecks) {
        function Set-ObjectOpacity([int]$Value) {
            # Drain prior posted selection gestures before sending synchronous text.
            $ack=[SnapZyTests.Native]::GetDlgItem($preview,202)
            [SnapZyTests.Native]::SendMessage($ack,0xf3,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
            [SnapZyTests.Native]::PostMessage($ack,0xf3,[IntPtr](1),[IntPtr]::Zero) | Out-Null
            Wait-For {([SnapZyTests.Native]::SendMessage($ack,0xf2,[IntPtr]::Zero,[IntPtr]::Zero).ToInt64()-band 4) -ne 0} 'Editor did not acknowledge selection before opacity.' | Out-Null
            [SnapZyTests.Native]::SendMessage($ack,0xf3,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
            $field=[SnapZyTests.Native]::GetDlgItem($preview,634)
            [SnapZyTests.Native]::WriteText($field,0xc,[IntPtr]::Zero,$Value.ToString()) | Out-Null
            [SnapZyTests.Native]::PostMessage($preview,0x111,[IntPtr](634-bor (512-shl 16)),$field) | Out-Null
            Wait-For {[SnapZyTests.Native]::Text($field) -eq $Value.ToString()} 'Opacity control did not update.' | Out-Null
        }
        function Assert-ResizePixel([string]$Path,[int]$X,[int]$Y,[Drawing.Color]$Expected) {
            $b=[Drawing.Bitmap]::FromFile($Path)
            try {if($b.GetPixel($X,$Y).ToArgb() -ne $Expected.ToArgb()){throw "Resize pixel mismatch at ($X,$Y) in $Path; actual=$($b.GetPixel($X,$Y)) expected=$Expected"}}
            finally {$b.Dispose()}
        }
        $v=Image-View $preview 160 80
        foreach($id in $(if($SkipShapeResize){@()}else{@(606,636,637,638)})) {
            Select-TestShape $preview $id
            Mouse $preview 0x201 ($v[0]+20) ($v[1]+20)
            Mouse $preview 0x202 ($v[0]+100) ($v[1]+60)
            $original=Save-Png $preview "resize-$id-original.png"
            Snapshot-Editor $preview "resize-$id-handles.png"
            foreach($xy in @(@(20,20),@(100,20),@(100,60),@(20,60),@(20,40),@(100,40),@(60,20),@(60,60))) {
                Assert-ResizePixel (Join-Path $runDirectory "resize-$id-handles.png") ($v[0]+$xy[0]) ($v[1]+$xy[1]) ([Drawing.Color]::White)
            }
            Mouse $preview 0x201 ($v[0]+100) ($v[1]+60)
            Mouse $preview 0x200 ($v[0]+120) ($v[1]+70)
            Snapshot-Editor $preview "resize-$id-live.png"
            Mouse $preview 0x202 ($v[0]+120) ($v[1]+70)
            $corner=Save-Png $preview "resize-$id-corner.png"
            # Filled ellipses antialias their boundary; sample one pixel inside.
            $inset=if($id -eq 638){1}else{0}
            Assert-ResizePixel $corner (119-$inset) 45 ([Drawing.Color]::Red)
            Click $preview 607
            if((Get-FileHash (Save-Png $preview "resize-$id-undo.png")).Hash -ne (Get-FileHash $original).Hash){throw 'Shape resize is not one Undo step.'}
            Click $preview 608
            if((Get-FileHash (Save-Png $preview "resize-$id-redo.png")).Hash -ne (Get-FileHash $corner).Hash){throw 'Shape resize Redo differs.'}
            Mouse $preview 0x201 ($v[0]+120) ($v[1]+45)
            Mouse $preview 0x202 ($v[0]+140) ($v[1]+45)
            Assert-ResizePixel (Save-Png $preview "resize-$id-side.png") (139-$inset) 45 ([Drawing.Color]::Red)
            Mouse $preview 0x201 ($v[0]+80) ($v[1]+20)
            Mouse $preview 0x202 ($v[0]+80) ($v[1]+10)
            $edge=Save-Png $preview "resize-$id-top.png"
            Assert-ResizePixel $edge 80 (10+$inset) ([Drawing.Color]::Red)
            Mouse $preview 0x201 ($v[0]+140) ($v[1]+40)
            Mouse $preview 0x200 ($v[0]+150) ($v[1]+40)
            [SnapZyTests.Native]::PostMessage($preview,0x100,[IntPtr](27),[IntPtr]::Zero) | Out-Null
            Wait-For {[SnapZyTests.Native]::MouseCapture($preview) -ne $preview} 'Resize Escape did not release capture.' | Out-Null
            if((Get-FileHash (Save-Png $preview "resize-$id-cancel.png")).Hash -ne (Get-FileHash $edge).Hash){throw 'Resize Escape changed the document.'}
            Click $preview 609
            if((Get-FileHash (Save-Png $preview "resize-$id-delete.png")).Hash -ne (Get-FileHash $png).Hash){throw 'Resize unexpectedly created another object.'}
        }
        if(-not $SkipShapeResize){$measurements.shape_resize='PASS'}
        Select-TestShape $preview 637
        Mouse $preview 0x201 ($v[0]+20) ($v[1]+20)
        Mouse $preview 0x202 ($v[0]+100) ($v[1]+60)
        $opaque=Save-Png $preview 'object-opaque.png'
        Set-ObjectOpacity 50
        $half=Save-Png $preview 'object-half.png'
        Assert-ResizePixel $half 60 40 ([Drawing.Color]::FromArgb(230,34,59))
        Click $preview 607
        if((Get-FileHash (Save-Png $preview 'object-opacity-undo.png')).Hash -ne (Get-FileHash $opaque).Hash){throw 'Object opacity Undo differs.'}
        Click $preview 608
        if((Get-FileHash (Save-Png $preview 'object-opacity-redo.png')).Hash -ne (Get-FileHash $half).Hash){throw 'Object opacity Redo differs.'}
        Set-ObjectOpacity 0
        if((Get-FileHash (Save-Png $preview 'object-zero.png')).Hash -ne (Get-FileHash $png).Hash){throw 'Opacity zero is not invisible.'}
        Click $preview 601
        Mouse $preview 0x201 ($v[0]+60) ($v[1]+40)
        Mouse $preview 0x202 ($v[0]+60) ($v[1]+40)
        Set-ObjectOpacity 50
        $final=Save-Png $preview 'object-selected-half.png'
        if((Get-FileHash $final).Hash -ne (Get-FileHash $half).Hash){throw 'Invisible shape could not be selected.'}
        Click $preview 201
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -eq 'Copied'} 'Object clipboard export failed.' | Out-Null
        $a=[Drawing.Bitmap]::FromFile($final);$copy=[Windows.Forms.Clipboard]::GetImage();$b=[Drawing.Bitmap]::new($copy)
        try {for($y=0;$y -lt 80;$y++){for($x=0;$x -lt 160;$x++){if($a.GetPixel($x,$y).ToArgb() -ne $b.GetPixel($x,$y).ToArgb()){throw 'Opacity clipboard pixels differ from PNG.'}}}}finally{$a.Dispose();$b.Dispose();$copy.Dispose()}
        Click $preview 609
        foreach($id in @(617,624)) {
            Click $preview $id
            Save-Png $preview "opacity-layout-$id.png" | Out-Null
            foreach($language in @('en','th')) {
                if($language -eq 'th'){Click $preview 614}
                Snapshot-Editor $preview "opacity-toolbar-$id-$language.png"
                $opacity=[SnapZyTests.Native]::Client([SnapZyTests.Native]::GetDlgItem($preview,634))
                foreach($control in @(620,621,622,623)) {
                    $h=[SnapZyTests.Native]::GetDlgItem($preview,$control)
                    if([SnapZyTests.Native]::IsWindowVisible($h)) {
                        $r=[SnapZyTests.Native]::Client($h)
                        if($opacity.Left -lt $r.Right -and $opacity.Right -gt $r.Left -and $opacity.Top -lt $r.Bottom -and $opacity.Bottom -gt $r.Top){throw 'Opacity overlaps text controls.'}
                    }
                }
            }
            Click $preview 614
        }
        Click $preview 603
        Set-ObjectOpacity 100
        Save-Png $preview 'resize-opacity-reset.png' | Out-Null
        $measurements.shape_resize_opacity='PASS'
    }
    if ($GeometryChecks) {
        function Assert-GeometryPixel([string]$Path,[int]$X,[int]$Y,[Drawing.Color]$Expected) {
            $b=[Drawing.Bitmap]::FromFile($Path)
            try {if($b.GetPixel($X,$Y).ToArgb() -ne $Expected.ToArgb()){throw "Geometry pixel differs: $Path ($X,$Y); expected=$Expected actual=$($b.GetPixel($X,$Y))"}}
            finally {$b.Dispose()}
        }
        function Select-Pattern([int]$Index) {
            Click $preview 639
            $menu=Wait-For {[SnapZyTests.Native]::Windows($process.Id,'#32768') | Where-Object {[SnapZyTests.Native]::IsWindowVisible($_)} | Select-Object -First 1} 'Stroke pattern menu missing.'
            for($i=0;$i -lt 6 -and [SnapZyTests.Native]::HighlightedMenuItem($menu) -ne $Index;$i++) {
                $old=[SnapZyTests.Native]::HighlightedMenuItem($menu)
                [SnapZyTests.Native]::PostMessage($preview,0x100,[IntPtr](40),[IntPtr]::Zero) | Out-Null
                Wait-For {$state=[SnapZyTests.Native]::HighlightedMenuItem($menu);$state -ge 0 -and $state -ne $old} 'Stroke menu did not advance.' | Out-Null
            }
            if([SnapZyTests.Native]::HighlightedMenuItem($menu) -ne $Index){throw 'Stroke menu selected the wrong item.'}
            [SnapZyTests.Native]::PostMessage($preview,0x100,[IntPtr](13),[IntPtr]::Zero) | Out-Null
            Wait-For {-not [SnapZyTests.Native]::IsWindowVisible($menu)} 'Stroke menu did not close.' | Out-Null
        }
        foreach($id in @(606,637,639)) {if([SnapZyTests.Native]::GetDlgItem($preview,$id) -eq [IntPtr]::Zero){throw "Geometry control $id missing."}}
        foreach($id in @(636,638)) {if([SnapZyTests.Native]::GetDlgItem($preview,$id) -ne [IntPtr]::Zero){throw "Ungrouped shape control $id still exists."}}
        $v=Image-View $preview 160 80
        foreach($id in @(606,636,637,638)) {
            Select-TestShape $preview $id
            Mouse $preview 0x201 ($v[0]+140) ($v[1]+60)
            Mouse $preview 0x200 ($v[0]+20) ($v[1]+20)
            Mouse $preview 0x202 ($v[0]+20) ($v[1]+20)
            $shape=Save-Png $preview ('geometry-shape-'+$id+'.png')
            Assert-GeometryPixel $shape 80 40 $(if($id -ge 637){[Drawing.Color]::Red}else{$fixture.BackColor})
            Assert-GeometryPixel $shape 21 21 $(if($id -eq 606 -or $id -eq 637){[Drawing.Color]::Red}else{$fixture.BackColor})
            if([SnapZyTests.Native]::IsWindowEnabled([SnapZyTests.Native]::GetDlgItem($preview,611)) -ne ($id -lt 637)){throw 'Filled shape width enablement is wrong.'}
            if([SnapZyTests.Native]::IsWindowEnabled([SnapZyTests.Native]::GetDlgItem($preview,639)) -ne ($id -lt 637)){throw 'Filled shape pattern enablement is wrong.'}
            Click $preview 607
            $undo=Save-Png $preview ('geometry-shape-'+$id+'-undo.png')
            if((Get-FileHash $undo).Hash -ne (Get-FileHash $png).Hash){throw 'Shape Undo failed to restore original pixels.'}
            Click $preview 608
            if((Get-FileHash (Save-Png $preview ('geometry-shape-'+$id+'-redo.png'))).Hash -ne (Get-FileHash $shape).Hash){throw 'Shape Redo differs.'}
            Click $preview 607
        }
        foreach($pair in @(@(606,1),@(637,1))) {
            Click $preview $pair[0]
            $menu=Wait-For {[SnapZyTests.Native]::Windows($process.Id,'#32768') | Where-Object {[SnapZyTests.Native]::IsWindowVisible($_)} | Select-Object -First 1} 'Remembered shape popup missing.'
            if(([SnapZyTests.Native]::MenuState($menu,$pair[1]) -band 8) -eq 0){throw 'Shape group did not remember ellipse independently.'}
            [SnapZyTests.Native]::PostMessage($preview,0x100,[IntPtr](27),[IntPtr]::Zero) | Out-Null
            Wait-For {-not [SnapZyTests.Native]::IsWindowVisible($menu)} 'Shape popup Escape failed.' | Out-Null
        }
        Click $preview 604
        Mouse $preview 0x201 ($v[0]+20) ($v[1]+60)
        Mouse $preview 0x202 ($v[0]+140) ($v[1]+60)
        $straight=Save-Png $preview 'geometry-straight.png'
        Snapshot-Editor $preview 'geometry-immediate-handle.png'
        Assert-GeometryPixel (Join-Path $runDirectory 'geometry-immediate-handle.png') ($v[0]+80) ($v[1]+60) ([Drawing.Color]::White)
        Click $preview 606
        $menu=Wait-For {[SnapZyTests.Native]::Windows($process.Id,'#32768') | Where-Object {[SnapZyTests.Native]::IsWindowVisible($_)} | Select-Object -First 1} 'Cancellation popup missing.'
        [SnapZyTests.Native]::PostMessage($preview,0x100,[IntPtr](27),[IntPtr]::Zero) | Out-Null
        Wait-For {-not [SnapZyTests.Native]::IsWindowVisible($menu)} 'Cancellation popup did not close.' | Out-Null
        if((Get-FileHash (Save-Png $preview 'geometry-popup-cancel.png')).Hash -ne (Get-FileHash $straight).Hash){throw 'Popup cancellation changed document.'}
        Mouse $preview 0x201 ($v[0]+80) ($v[1]+60)
        Mouse $preview 0x200 ($v[0]+80) ($v[1]+30)
        Wait-For {[SnapZyTests.Native]::MouseCapture($preview) -eq $preview} 'Curve drag did not start.' | Out-Null
        Start-Sleep -Milliseconds 100
        Snapshot-Editor $preview 'geometry-curve-draft.png'
        Assert-GeometryPixel (Join-Path $runDirectory 'geometry-curve-draft.png') ($v[0]+60) ($v[1]+60) $fixture.BackColor
        Mouse $preview 0x202 ($v[0]+80) ($v[1]+30)
        $curved=Save-Png $preview 'geometry-curved.png'
        Assert-GeometryPixel $curved 80 30 ([Drawing.Color]::Red)
        Assert-GeometryPixel $curved 60 60 $fixture.BackColor
        Click $preview 607
        if((Get-FileHash (Save-Png $preview 'geometry-curve-undo.png')).Hash -ne (Get-FileHash $straight).Hash){throw 'Curve drag is not one Undo edit.'}
        Click $preview 608
        if((Get-FileHash (Save-Png $preview 'geometry-curve-redo.png')).Hash -ne (Get-FileHash $curved).Hash){throw 'Curve Redo differs.'}
        Mouse $preview 0x201 ($v[0]+20) ($v[1]+15)
        Mouse $preview 0x202 ($v[0]+140) ($v[1]+15)
        Assert-GeometryPixel (Save-Png $preview 'geometry-continue-drawing.png') 60 15 ([Drawing.Color]::Red)
        Click $preview 607
        if((Get-FileHash (Save-Png $preview 'geometry-continue-undo.png')).Hash -ne (Get-FileHash $curved).Hash){throw 'Continued drawing did not stay in Line mode.'}
        # Undo of the new object follows existing selection rules; reselect the older curve.
        Click $preview 601
        Mouse $preview 0x201 ($v[0]+40) ($v[1]+43)
        Mouse $preview 0x202 ($v[0]+40) ($v[1]+43)
        Mouse $preview 0x201 ($v[0]+80) ($v[1]+30)
        Mouse $preview 0x200 ($v[0]+80) ($v[1]+45)
        [SnapZyTests.Native]::PostMessage($preview,0x100,[IntPtr](27),[IntPtr]::Zero) | Out-Null
        Mouse $preview 0x202 ($v[0]+80) ($v[1]+45)
        if((Get-FileHash (Save-Png $preview 'geometry-curve-cancel.png')).Hash -ne (Get-FileHash $curved).Hash){throw 'Escape did not cancel curve drag.'}
        Mouse $preview 0x203 ($v[0]+80) ($v[1]+30)
        Mouse $preview 0x202 ($v[0]+80) ($v[1]+30)
        if((Get-FileHash (Save-Png $preview 'geometry-curve-reset.png')).Hash -ne (Get-FileHash $straight).Hash){throw 'Double-click did not straighten curve.'}
        Click $preview 607
        if((Get-FileHash (Save-Png $preview 'geometry-reset-undo.png')).Hash -ne (Get-FileHash $curved).Hash){throw 'Straighten Undo differs.'}
        Mouse $preview 0x203 ($v[0]+80) ($v[1]+30)
        Mouse $preview 0x202 ($v[0]+80) ($v[1]+30)
        Save-Png $preview 'geometry-reset-for-patterns.png' | Out-Null
        foreach($index in @(1,2)) {
            Select-Pattern $index
            $patterned=Save-Png $preview ('geometry-pattern-'+$index+'.png')
            Assert-GeometryPixel $patterned $(if($index -eq 1){50}else{27}) 60 $fixture.BackColor
            Assert-GeometryPixel $patterned $(if($index -eq 1){60}else{35}) 60 ([Drawing.Color]::Red)
        }
        Select-Pattern 0
        Save-Png $preview 'geometry-solid-again.png' | Out-Null
        Mouse $preview 0x201 ($v[0]+80) ($v[1]+60)
        Mouse $preview 0x202 ($v[0]+80) ($v[1]+30)
        Save-Png $preview 'geometry-curve-for-layer.png' | Out-Null
        Select-TestShape $preview 637
        Set-LayerTestColor $preview 4
        Mouse $preview 0x201 ($v[0]+60) ($v[1]+20)
        Mouse $preview 0x202 ($v[0]+100) ($v[1]+40)
        Save-Png $preview 'geometry-top-layer.png' | Out-Null
        Click $preview 601
        Mouse $preview 0x201 ($v[0]+40) ($v[1]+43)
        Mouse $preview 0x202 ($v[0]+40) ($v[1]+43)
        Mouse $preview 0x201 ($v[0]+80) ($v[1]+30)
        Mouse $preview 0x200 ($v[0]+80) ($v[1]+25)
        Start-Sleep -Milliseconds 100
        Snapshot-Editor $preview 'geometry-layered-draft.png'
        Assert-GeometryPixel (Join-Path $runDirectory 'geometry-layered-draft.png') ($v[0]+70) ($v[1]+30) ([Drawing.Color]::FromArgb(0,120,255))
        Mouse $preview 0x202 ($v[0]+80) ($v[1]+25)
        $final=Save-Png $preview 'geometry-layered-final.png'
        Assert-GeometryPixel $final 80 25 ([Drawing.Color]::FromArgb(0,120,255))
        Click $preview 201
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -eq 'Copied'} 'Geometry clipboard copy failed.' | Out-Null
        $a=[Drawing.Bitmap]::FromFile($final);$copy=[Windows.Forms.Clipboard]::GetImage();$b=[Drawing.Bitmap]::new($copy)
        try {for($y=0;$y -lt 80;$y++){for($x=0;$x -lt 160;$x++){if($a.GetPixel($x,$y).ToArgb() -ne $b.GetPixel($x,$y).ToArgb()){throw 'Geometry clipboard differs from PNG.'}}}}finally{$a.Dispose();$b.Dispose();$copy.Dispose()}
        foreach($language in @('en','th')) {
            if($language -eq 'th'){Click $preview 614}
            Click $preview 615
            $help=Wait-For {Window-Of '#32770'} 'Geometry Help missing.'
            $text=([SnapZyTests.Native]::Children($help) | ForEach-Object {[SnapZyTests.Native]::Text($_)}) -join ' '
            if($language -eq 'en' -and $text -notmatch 'Double-click the handle to straighten'){throw 'English curve Help missing.'}
            if($language -eq 'th' -and $text -notmatch 'คืนเส้นตรง'){throw 'Thai curve Help missing.'}
            Dismiss-Message $help 1
            Snapshot-Editor $preview ('geometry-toolbar-'+$language+'.png')
        }
        Click $preview 614
        for($i=0;$i -lt 32;$i++){Click $preview 607}
        Click $preview 605
        Set-LayerTestColor $preview 0
        Mouse $preview 0x201 ($v[0]+20) ($v[1]+60)
        Mouse $preview 0x202 ($v[0]+140) ($v[1]+60)
        Save-Png $preview 'geometry-arrow-straight.png' | Out-Null
        Mouse $preview 0x201 ($v[0]+80) ($v[1]+60)
        Mouse $preview 0x202 ($v[0]+80) ($v[1]+30)
        Select-Pattern 2
        $arrow=Save-Png $preview 'geometry-curved-dotted-arrow.png'
        Assert-GeometryPixel $arrow 135 48 ([Drawing.Color]::Red)
        Click $preview 601
        Mouse $preview 0x201 ($v[0]+40) ($v[1]+43)
        Mouse $preview 0x202 ($v[0]+40) ($v[1]+43)
        Mouse $preview 0x201 ($v[0]+40) ($v[1]+43)
        Mouse $preview 0x202 ($v[0]+50) ($v[1]+48)
        Save-Png $preview 'geometry-arrow-moved.png' | Out-Null
        Click $preview 607
        if((Get-FileHash (Save-Png $preview 'geometry-arrow-move-undo.png')).Hash -ne (Get-FileHash $arrow).Hash){throw 'Curved arrow movement changed shape or pattern.'}
        Click $preview 609
        Save-Png $preview 'geometry-arrow-deleted.png' | Out-Null
        Click $preview 607
        if((Get-FileHash (Save-Png $preview 'geometry-arrow-delete-undo.png')).Hash -ne (Get-FileHash $arrow).Hash){throw 'Curved arrow Delete Undo differs.'}
        $long=[Drawing.Bitmap]::new(100,8000);$g=[Drawing.Graphics]::FromImage($long)
        try {$g.Clear([Drawing.Color]::White);$longPath=Join-Path $runDirectory 'geometry-long-input.png';$long.Save($longPath,[Drawing.Imaging.ImageFormat]::Png)}finally{$g.Dispose();$long.Dispose()}
        Import-TestImage $preview $longPath
        Save-Png $preview 'geometry-long-before.png' | Out-Null
        Click $preview 605
        $r=[SnapZyTests.Native]::Client($preview)
        $ratio=[Math]::Min(1.0,[Math]::Min(($r.Right-$r.Left-32)/160,($r.Bottom-$r.Top-175)/8000))
        $lx=($r.Right-$r.Left-160*$ratio)/2
        $ly=145+($r.Bottom-$r.Top-175-8000*$ratio)/2
        Mouse $preview 0x201 ([int]($lx+20*$ratio)) ([int]($ly+1000*$ratio))
        Mouse $preview 0x202 ([int]($lx+20*$ratio)) ([int]($ly+7000*$ratio))
        Save-Png $preview 'geometry-long-straight.png' | Out-Null
        Click $preview 601
        Mouse $preview 0x201 ([int]($lx+20*$ratio)) ([int]($ly+2000*$ratio))
        Mouse $preview 0x202 ([int]($lx+20*$ratio)) ([int]($ly+2000*$ratio))
        Mouse $preview 0x201 ([int]($lx+20*$ratio)) ([int]($ly+4000*$ratio))
        $clock=[Diagnostics.Stopwatch]::StartNew()
        for($i=0;$i -lt 40;$i++){Mouse $preview 0x200 ([int]($lx+60*$ratio)) ([int]($ly+(4000+20*($i%5))*$ratio))}
        Wait-For {[SnapZyTests.Native]::MouseCapture($preview) -eq $preview} 'Long curve gesture did not start.' | Out-Null
        $ack=[SnapZyTests.Native]::GetDlgItem($preview,202)
        [SnapZyTests.Native]::SendMessage($ack,0xf3,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
        [SnapZyTests.Native]::PostMessage($ack,0xf3,[IntPtr](1),[IntPtr]::Zero) | Out-Null
        Wait-For {([SnapZyTests.Native]::SendMessage($ack,0xf2,[IntPtr]::Zero,[IntPtr]::Zero).ToInt64()-band 4) -ne 0} 'Long curve did not acknowledge queued moves.' | Out-Null
        $measurements.curve_long_move_queue_ms=[Math]::Round($clock.Elapsed.TotalMilliseconds,3)
        [SnapZyTests.Native]::SendMessage($ack,0xf3,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
        $clock.Restart();Snapshot-Editor $preview 'geometry-long-curve-draft.png'
        $measurements.curve_long_snapshot_ms=[Math]::Round($clock.Elapsed.TotalMilliseconds,3)
        Mouse $preview 0x202 ([int]($lx+60*$ratio)) ([int]($ly+4080*$ratio))
        $longResult=Save-Png $preview 'geometry-long-final.png'
        $b=[Drawing.Bitmap]::FromFile($longResult)
        try {if($b.Width -ne 160 -or $b.Height -ne 8000){throw 'Curve drag changed long-image output resolution.'}}finally{$b.Dispose()}
        for($i=0;$i -lt 32;$i++){Click $preview 607}
        Click $preview 603
        $reset=Save-Png $preview 'geometry-reset.png'
        if((Get-FileHash $reset).Hash -ne (Get-FileHash $png).Hash){throw 'Geometry checks did not restore original capture.'}
        $measurements.geometry='PASS'
    }
    if ($HighlighterChecks) {
        foreach($id in @(633,634)) {
            if([SnapZyTests.Native]::GetDlgItem($preview,$id) -eq [IntPtr]::Zero){throw "Highlighter control $id is missing."}
        }
        function Set-HighlightOpacity([int]$Value) {
            Save-Png $preview ('opacity-before-'+[guid]::NewGuid().ToString('N')+'.png') | Out-Null
            $field=[SnapZyTests.Native]::GetDlgItem($preview,634)
            [SnapZyTests.Native]::WriteText($field,0xc,[IntPtr]::Zero,$Value.ToString()) | Out-Null
            [SnapZyTests.Native]::PostMessage($preview,0x111,[IntPtr](634-bor (512-shl 16)),$field) | Out-Null
        }
        function Assert-HighlightPixel([string]$Path,[int]$Y,[Drawing.Color]$Expected) {
            $b=[Drawing.Bitmap]::FromFile($Path)
            try {
                $actual=$b.GetPixel(60,$Y)
                if($b.Width -ne 160 -or $b.Height -ne 80 -or $actual.ToArgb() -ne $Expected.ToArgb()){throw "Highlighter pixel differs: $Path; expected=$Expected actual=$actual"}
            }finally{$b.Dispose()}
        }
        Click $preview 633
        Save-Png $preview 'highlight-defaults.png' | Out-Null
        if([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,611)) -ne '24' -or [SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,634)) -ne '40'){throw 'Highlighter defaults must be 24 px and 40 percent.'}
        if(-not [SnapZyTests.Native]::IsWindowVisible([SnapZyTests.Native]::GetDlgItem($preview,634))){throw 'Highlighter opacity is not visible.'}
        Snapshot-Editor $preview 'highlight-toolbar-en.png'
        $v=Image-View $preview 160 80
        Mouse $preview 0x201 ($v[0]+20) ($v[1]+30)
        Mouse $preview 0x200 ($v[0]+130) ($v[1]+30)
        Wait-For {[SnapZyTests.Native]::MouseCapture($preview) -eq $preview} 'Highlighter did not begin a live gesture.' | Out-Null
        Start-Sleep -Milliseconds 100
        Snapshot-Editor $preview 'highlight-draft.png'
        Mouse $preview 0x202 ($v[0]+130) ($v[1]+30)
        $highlight=Save-Png $preview 'highlight-yellow.png'
        $yellow=[Drawing.Color]::FromArgb(204,68,88)
        Assert-HighlightPixel $highlight 30 $yellow
        Snapshot-Editor $preview 'highlight-final.png'
        $draft=[Drawing.Bitmap]::FromFile((Join-Path $runDirectory 'highlight-draft.png'))
        $done=[Drawing.Bitmap]::FromFile((Join-Path $runDirectory 'highlight-final.png'))
        try {
            for($y=19;$y -le 41;$y++){for($x=25;$x -lt 120;$x++){
                if($draft.GetPixel($v[0]+$x,$v[1]+$y).ToArgb() -ne $done.GetPixel($v[0]+$x,$v[1]+$y).ToArgb()){throw 'Highlighter live pixels differ from the finished stroke.'}
            }}
            if($draft.GetPixel($v[0]+60,$v[1]+30).ToArgb() -ne $yellow.ToArgb()){throw 'Highlighter live stroke did not use multiply blending.'}
        }finally{$draft.Dispose();$done.Dispose()}
        Set-HighlightOpacity 100
        Assert-HighlightPixel (Save-Png $preview 'highlight-opaque.png') 30 ([Drawing.Color]::FromArgb(204,65,41))
        Click $preview 607
        $undo=Save-Png $preview 'highlight-opacity-undo.png'
        if((Get-FileHash $undo).Hash -ne (Get-FileHash $highlight).Hash -or [SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,634)) -ne '40'){throw 'Highlighter opacity Undo did not restore pixels and control.'}
        Click $preview 608
        Assert-HighlightPixel (Save-Png $preview 'highlight-opacity-redo.png') 30 ([Drawing.Color]::FromArgb(204,65,41))
        Set-HighlightOpacity 0
        Assert-HighlightPixel (Save-Png $preview 'highlight-transparent.png') 30 $fixture.BackColor
        Set-HighlightOpacity 40
        Set-Parameter $preview 18
        Set-LayerTestColor $preview 4
        Assert-HighlightPixel (Save-Png $preview 'highlight-blue.png') 30 ([Drawing.Color]::FromArgb(122,54,119))
        Click $preview 601
        Mouse $preview 0x201 ($v[0]+60) ($v[1]+30)
        Mouse $preview 0x200 ($v[0]+60) ($v[1]+50)
        Mouse $preview 0x202 ($v[0]+60) ($v[1]+50)
        $moved=Save-Png $preview 'highlight-moved.png'
        Assert-HighlightPixel $moved 30 $fixture.BackColor
        Assert-HighlightPixel $moved 50 ([Drawing.Color]::FromArgb(122,54,119))
        Click $preview 609
        Assert-HighlightPixel (Save-Png $preview 'highlight-deleted.png') 50 $fixture.BackColor
        Click $preview 607
        $restored=Save-Png $preview 'highlight-delete-undo.png'
        if((Get-FileHash $restored).Hash -ne (Get-FileHash $moved).Hash){throw 'Highlighter Delete Undo differs.'}
        Click $preview 201
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -eq 'Copied'} 'Highlighter clipboard export failed.' | Out-Null
        $a=[Drawing.Bitmap]::FromFile($moved);$copy=[Windows.Forms.Clipboard]::GetImage();$b=[Drawing.Bitmap]::new($copy)
        try {for($y=0;$y -lt 80;$y++){for($x=0;$x -lt 160;$x++){if($a.GetPixel($x,$y).ToArgb() -ne $b.GetPixel($x,$y).ToArgb()){throw 'Highlighter clipboard pixels differ from PNG.'}}}}finally{$a.Dispose();$b.Dispose();$copy.Dispose()}
        Click $preview 603
        Save-Png $preview 'highlight-pen-settings.png' | Out-Null
        if([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,611)) -ne '5' -or [SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,634)) -ne '100'){throw 'Highlighter changed the normal Pen settings.'}
        Mouse $preview 0x201 ($v[0]+20) ($v[1]+15)
        Mouse $preview 0x202 ($v[0]+130) ($v[1]+15)
        Assert-HighlightPixel (Save-Png $preview 'highlight-independent-red-pen.png') 15 ([Drawing.Color]::Red)
        Click $preview 633
        Save-Png $preview 'highlight-remembered.png' | Out-Null
        if([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,611)) -ne '18' -or [SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,634)) -ne '40'){throw 'Highlighter custom settings were not remembered independently.'}
        foreach($language in @('en','th')) {
            if($language -eq 'th'){Click $preview 614}
            Click $preview 615
            $help=Wait-For {Window-Of '#32770'} 'Highlighter Help is missing.'
            $text=([SnapZyTests.Native]::Children($help) | ForEach-Object {[SnapZyTests.Native]::Text($_)}) -join ' '
            if($text -notmatch '24 px' -or $text -notmatch '40%'){throw "Highlighter Help defaults are missing in $language."}
            Dismiss-Message $help 1
            Snapshot-Editor $preview ('highlight-toolbar-'+$language+'.png')
        }
        Click $preview 614
        Mouse $preview 0x201 ($v[0]+20) ($v[1]+70)
        Mouse $preview 0x200 ($v[0]+130) ($v[1]+70)
        [SnapZyTests.Native]::PostMessage($preview,0x100,[IntPtr](27),[IntPtr]::Zero) | Out-Null
        Mouse $preview 0x202 ($v[0]+130) ($v[1]+70)
        Assert-HighlightPixel (Save-Png $preview 'highlight-cancelled.png') 70 $fixture.BackColor
        $longImage=[Drawing.Bitmap]::new(100,8000)
        $g=[Drawing.Graphics]::FromImage($longImage)
        try {
            $g.Clear([Drawing.Color]::White)
            for($y=0;$y -lt 8000;$y+=40){$g.FillRectangle([Drawing.Brushes]::Black,20,$y,60,4)}
            $longPath=Join-Path $runDirectory 'highlight-long-input.png'
            $longImage.Save($longPath,[Drawing.Imaging.ImageFormat]::Png)
        }finally{$g.Dispose();$longImage.Dispose()}
        Import-TestImage $preview $longPath
        Save-Png $preview 'highlight-long-before.png' | Out-Null
        Click $preview 633
        $r=[SnapZyTests.Native]::Client($preview)
        $ratio=[Math]::Min(1.0,[Math]::Min(($r.Right-$r.Left-32)/160,($r.Bottom-$r.Top-175)/8000))
        $lx=($r.Right-$r.Left-160*$ratio)/2
        $ly=145+($r.Bottom-$r.Top-175-8000*$ratio)/2
        $batch=[Diagnostics.Stopwatch]::StartNew()
        Mouse $preview 0x201 ([int]($lx+40*$ratio)) ([int]($ly+1000*$ratio))
        for($i=1;$i -le 40;$i++){Mouse $preview 0x200 ([int]($lx+60*$ratio)) ([int]($ly+(1000+75*$i)*$ratio))}
        Wait-For {[SnapZyTests.Native]::MouseCapture($preview) -eq $preview} 'Long-image highlighter gesture did not start.' | Out-Null
        $ack=[SnapZyTests.Native]::GetDlgItem($preview,202)
        [SnapZyTests.Native]::SendMessage($ack,0xf3,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
        [SnapZyTests.Native]::PostMessage($ack,0xf3,[IntPtr](1),[IntPtr]::Zero) | Out-Null
        Wait-For {([SnapZyTests.Native]::SendMessage($ack,0xf2,[IntPtr]::Zero,[IntPtr]::Zero).ToInt64()-band 4) -ne 0} 'Long-image highlighter did not acknowledge all draft moves.' | Out-Null
        [SnapZyTests.Native]::SendMessage($ack,0xf3,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
        $measurements.highlighter_long_draft_batch_ms=[Math]::Round($batch.Elapsed.TotalMilliseconds,3)
        $ping=[Diagnostics.Stopwatch]::StartNew();$reply=[IntPtr]::Zero
        if([SnapZyTests.Native]::SendMessageTimeout($preview,0,[IntPtr]::Zero,[IntPtr]::Zero,2,1000,[ref]$reply) -eq [IntPtr]::Zero){throw 'Highlighter draft blocked the long-image editor.'}
        $measurements.highlighter_long_draft_ping_ms=[Math]::Round($ping.Elapsed.TotalMilliseconds,3)
        Snapshot-Editor $preview 'highlight-long-draft.png'
        Mouse $preview 0x202 ([int]($lx+60*$ratio)) ([int]($ly+4000*$ratio))
        $longResult=Save-Png $preview 'highlight-long-finished.png'
        $b=[Drawing.Bitmap]::FromFile($longResult)
        try {if($b.Width -ne 160 -or $b.Height -ne 8000){throw 'Highlighter changed long-image output resolution.'}}finally{$b.Dispose()}
        $measurements.highlighter_long_dimensions='160 x 8000'
        for($i=0;$i -lt 32;$i++){Click $preview 607}
        Click $preview 603
        $reset=Save-Png $preview 'highlight-reset.png'
        if((Get-FileHash $reset).Hash -ne (Get-FileHash $png).Hash){throw 'Highlighter checks did not restore the original capture.'}
        $measurements.highlighter='PASS'
    }
    if ($MultiImageChecks) {
        foreach($id in @(629,630,631)) {
            if([SnapZyTests.Native]::GetDlgItem($preview,$id) -eq [IntPtr]::Zero){throw "Multi-image command $id is missing."}
        }
        $input=[Drawing.Bitmap]::new(40,20)
        $g=[Drawing.Graphics]::FromImage($input)
        try {
            $g.Clear([Drawing.Color]::Blue)
            $g.FillRectangle([Drawing.Brushes]::Red,0,0,20,20)
            foreach($format in @('png','jpg','bmp')) {
                $kind=switch($format){'png'{[Drawing.Imaging.ImageFormat]::Png}'jpg'{[Drawing.Imaging.ImageFormat]::Jpeg}'bmp'{[Drawing.Imaging.ImageFormat]::Bmp}}
                $input.Save((Join-Path $runDirectory ('import.'+$format)),$kind)
            }
        }finally{$g.Dispose();$input.Dispose()}
        Import-TestImage $preview (Join-Path $runDirectory 'import.png')
        $added=Save-Png $preview 'image-added.png'
        $b=[Drawing.Bitmap]::FromFile($added)
        try {
            if($b.Width -ne 160 -or $b.Height -ne 80 -or $b.GetPixel(10,10).ToArgb() -ne [Drawing.Color]::Red.ToArgb() -or $b.GetPixel(30,10).ToArgb() -ne [Drawing.Color]::Blue.ToArgb()){throw 'Imported PNG pixels or canvas dimensions changed.'}
        }finally{$b.Dispose()}
        $v=Image-View $preview 160 80
        Mouse $preview 0x201 ($v[0]+10) ($v[1]+10)
        Mouse $preview 0x200 ($v[0]+70) ($v[1]+40)
        Mouse $preview 0x202 ($v[0]+70) ($v[1]+40)
        $moved=Save-Png $preview 'image-moved.png'
        $b=[Drawing.Bitmap]::FromFile($moved)
        try {if($b.GetPixel(65,35).ToArgb() -ne [Drawing.Color]::Red.ToArgb() -or $b.GetPixel(95,45).ToArgb() -ne [Drawing.Color]::Blue.ToArgb()){throw 'Image move did not preserve its source pixels.'}}finally{$b.Dispose()}
        Mouse $preview 0x201 ($v[0]+100) ($v[1]+50)
        Mouse $preview 0x200 ($v[0]+140) ($v[1]+70)
        Snapshot-Editor $preview 'image-resize-draft.png'
        Mouse $preview 0x202 ($v[0]+140) ($v[1]+70)
        $resized=Save-Png $preview 'image-resized.png'
        $b=[Drawing.Bitmap]::FromFile($resized)
        try {if($b.GetPixel(65,35).ToArgb() -ne [Drawing.Color]::Red.ToArgb() -or $b.GetPixel(135,65).ToArgb() -ne [Drawing.Color]::Blue.ToArgb() -or $b.GetPixel(145,75).ToArgb() -ne $fixture.BackColor.ToArgb()){throw 'Image corner resize distorted the expected fixed-anchor bounds.'}}finally{$b.Dispose()}
        Click $preview 607
        $undo=Save-Png $preview 'image-resize-undo.png'
        if((Get-FileHash $undo).Hash -ne (Get-FileHash $moved).Hash){throw 'Image resize Undo did not restore exact pixels.'}
        Click $preview 608
        $redo=Save-Png $preview 'image-resize-redo.png'
        if((Get-FileHash $redo).Hash -ne (Get-FileHash $resized).Hash){throw 'Image resize Redo differs.'}
        Mouse $preview 0x201 ($v[0]+140) ($v[1]+70)
        Mouse $preview 0x200 ($v[0]+150) ($v[1]+75)
        [SnapZyTests.Native]::PostMessage($preview,0x100,[IntPtr](27),[IntPtr]::Zero) | Out-Null
        Mouse $preview 0x202 ($v[0]+150) ($v[1]+75)
        $cancelled=Save-Png $preview 'image-resize-cancelled.png'
        if((Get-FileHash $cancelled).Hash -ne (Get-FileHash $resized).Hash){throw 'Image resize cancellation changed the document.'}
        Click $preview 631
        $entry=Wait-For {Window-Of 'SnapZyRustPreview.Text'} 'Canvas dialog missing.'
        foreach($pair in @(@(701,'240'),@(702,'140'))) {
            [SnapZyTests.Native]::WriteText([SnapZyTests.Native]::GetDlgItem($entry,$pair[0]),0xc,[IntPtr]::Zero,$pair[1]) | Out-Null
        }
        Click $entry 1
        Wait-For {-not (Window-Of 'SnapZyRustPreview.Text')} 'Canvas expansion did not close.' | Out-Null
        $expanded=Save-Png $preview 'canvas-expanded.png'
        $b=[Drawing.Bitmap]::FromFile($expanded)
        try {if($b.Width -ne 240 -or $b.Height -ne 140 -or $b.GetPixel(230,130).ToArgb() -ne [Drawing.Color]::White.ToArgb() -or $b.GetPixel(135,65).ToArgb() -ne [Drawing.Color]::Blue.ToArgb()){throw 'Canvas expansion lost objects or did not add white pixels.'}}finally{$b.Dispose()}
        $clip=[Drawing.Bitmap]::new(20,10)
        $g=[Drawing.Graphics]::FromImage($clip)
        try {$g.Clear([Drawing.Color]::Lime);[Windows.Forms.Clipboard]::SetImage($clip)}finally{$g.Dispose();$clip.Dispose()}
        Click $preview 630
        Save-Png $preview 'image-pasted.png' | Out-Null
        $v=Image-View $preview 240 140
        Mouse $preview 0x201 ($v[0]+10) ($v[1]+5)
        Mouse $preview 0x202 ($v[0]+95) ($v[1]+50)
        $stack=Save-Png $preview 'images-overlapping.png'
        $b=[Drawing.Bitmap]::FromFile($stack)
        try {if($b.GetPixel(90,48).ToArgb() -ne [Drawing.Color]::Lime.ToArgb()){throw 'Clipboard image paste/move did not create a new top layer.'}}finally{$b.Dispose()}
        Click $preview 628
        $back=Save-Png $preview 'image-back.png'
        $b=[Drawing.Bitmap]::FromFile($back)
        try {if($b.GetPixel(90,48).ToArgb() -ne [Drawing.Color]::Red.ToArgb()){throw 'Image ordering differs from annotation stack.'}}finally{$b.Dispose()}
        Click $preview 607
        $restored=Save-Png $preview 'image-order-undo.png'
        if((Get-FileHash $restored).Hash -ne (Get-FileHash $stack).Hash){throw 'Image layer Undo differs.'}
        Snapshot-Editor $preview 'multi-image-editor.png'
        Click $preview 201
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -eq 'Copied'} 'Multi-image copy failed.' | Out-Null
        $a=[Drawing.Bitmap]::FromFile($stack);$copy=[Windows.Forms.Clipboard]::GetImage();$b=[Drawing.Bitmap]::new($copy)
        try {for($y=0;$y -lt 140;$y++){for($x=0;$x -lt 240;$x++){if($a.GetPixel($x,$y).ToArgb() -ne $b.GetPixel($x,$y).ToArgb()){throw 'Multi-image clipboard and PNG differ.'}}}}finally{$a.Dispose();$b.Dispose();$copy.Dispose()}
        foreach($format in @('jpg','bmp')) {
            Import-TestImage $preview (Join-Path $runDirectory ('import.'+$format))
            $path=Save-Png $preview ('image-'+$format+'.png')
            $b=[Drawing.Bitmap]::FromFile($path)
            try {if($b.GetPixel(5,5).R -lt 230 -or $b.GetPixel(35,5).B -lt 230){throw "File import failed for $format."}}finally{$b.Dispose()}
            Click $preview 607
            $after=Save-Png $preview ('image-'+$format+'-undo.png')
            if((Get-FileHash $after).Hash -ne (Get-FileHash $stack).Hash){throw "Undo import for $format differs."}
        }
        Mouse $preview 0x201 ($v[0]+95) ($v[1]+50)
        Mouse $preview 0x202 ($v[0]+95) ($v[1]+50)
        Click $preview 609
        $deleted=Save-Png $preview 'image-deleted.png'
        if((Get-FileHash $deleted).Hash -eq (Get-FileHash $stack).Hash){throw 'Image Delete did not change pixels.'}
        Click $preview 607
        Save-Png $preview 'image-delete-undo.png' | Out-Null
        $large=[Drawing.Bitmap]::new(300,200)
        $g=[Drawing.Graphics]::FromImage($large)
        try {$g.Clear([Drawing.Color]::Yellow);$large.Save((Join-Path $runDirectory 'larger-than-canvas.png'),[Drawing.Imaging.ImageFormat]::Png)}finally{$g.Dispose();$large.Dispose()}
        Import-TestImage $preview (Join-Path $runDirectory 'larger-than-canvas.png')
        $largePng=Save-Png $preview 'image-auto-expanded.png'
        $b=[Drawing.Bitmap]::FromFile($largePng)
        try {if($b.Width -ne 300 -or $b.Height -ne 200 -or $b.GetPixel(299,199).ToArgb() -ne [Drawing.Color]::Yellow.ToArgb()){throw 'Larger image was reduced instead of expanding the canvas.'}}finally{$b.Dispose()}
        Click $preview 607
        $autoUndo=Save-Png $preview 'image-auto-expanded-undo.png'
        if((Get-FileHash $autoUndo).Hash -ne (Get-FileHash $stack).Hash){throw 'Larger image insertion and canvas growth were not one Undo step.'}
        $oversize=[Drawing.Bitmap]::new(3000,3000)
        try {$oversize.Save((Join-Path $runDirectory 'oversize.png'),[Drawing.Imaging.ImageFormat]::Png)}finally{$oversize.Dispose()}
        Import-TestImage $preview (Join-Path $runDirectory 'oversize.png') -ExpectError
        $afterError=Save-Png $preview 'image-oversize-rejected.png'
        if((Get-FileHash $afterError).Hash -ne (Get-FileHash $stack).Hash){throw 'Rejected large image changed the document.'}
        [IO.File]::WriteAllBytes((Join-Path $runDirectory 'broken.png'),[byte[]]@(1,2,3,4))
        Import-TestImage $preview (Join-Path $runDirectory 'broken.png') -ExpectError
        $afterError=Save-Png $preview 'image-broken-rejected.png'
        if((Get-FileHash $afterError).Hash -ne (Get-FileHash $stack).Hash){throw 'Rejected invalid image changed the document.'}
        for($i=0;$i -lt 64;$i++){Click $preview 607}
        $reset=Save-Png $preview 'multi-image-reset.png'
        if((Get-FileHash $reset).Hash -ne (Get-FileHash $png).Hash){throw 'Multi-image history did not restore original capture and dimensions.'}
        $v=Image-View $preview 160 80
        Click $preview 602
        Mouse $preview 0x201 ($v[0]+10) ($v[1]+10)
        Mouse $preview 0x202 ($v[0]+90) ($v[1]+50)
        [SnapZyTests.Native]::PostMessage($preview,0x100,[IntPtr](13),[IntPtr]::Zero) | Out-Null
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -eq '80 x 40 px'} 'Regrowth fixture crop did not apply.' | Out-Null
        Save-Png $preview 'before-canvas-regrowth.png' | Out-Null
        Click $preview 631
        $entry=Wait-For {Window-Of 'SnapZyRustPreview.Text'} 'Regrowth canvas dialog missing.'
        foreach($pair in @(@(701,'160'),@(702,'80'))) {
            [SnapZyTests.Native]::WriteText([SnapZyTests.Native]::GetDlgItem($entry,$pair[0]),0xc,[IntPtr]::Zero,$pair[1]) | Out-Null
        }
        Click $entry 1
        Wait-For {-not (Window-Of 'SnapZyRustPreview.Text')} 'Canvas regrowth did not close.' | Out-Null
        $regrown=Save-Png $preview 'canvas-regrowth-white.png'
        $b=[Drawing.Bitmap]::FromFile($regrown)
        try {if($b.Width -ne 160 -or $b.Height -ne 80 -or $b.GetPixel(150,70).ToArgb() -ne [Drawing.Color]::White.ToArgb()){throw 'Canvas expansion revealed cropped source pixels.'}}finally{$b.Dispose()}
        Click $preview 607
        Click $preview 607
        $restored=Save-Png $preview 'canvas-regrowth-undo.png'
        if((Get-FileHash $restored).Hash -ne (Get-FileHash $png).Hash){throw 'Crop/expansion Undo lost original capture.'}
        $measurements.multi_image='PASS'
    }
    if ($LayerChecks) {
        foreach($id in @(625,626,627,628)) {
            if([SnapZyTests.Native]::GetDlgItem($preview,$id) -eq [IntPtr]::Zero){throw "Layer order control $id is missing."}
        }
        foreach($id in @(625,626,627,628)) {
            if([SnapZyTests.Native]::IsWindowEnabled([SnapZyTests.Native]::GetDlgItem($preview,$id))){throw 'Layer command enabled without selection.'}
        }
        $r=[SnapZyTests.Native]::Client($preview)
        $vx=[int](($r.Right-$r.Left-160)/2)
        $vy=145+[int](($r.Bottom-$r.Top-175-80)/2)
        foreach($colorIndex in @(0,3,4)) {
            Click $preview 604
            Set-LayerTestColor $preview $colorIndex
            Mouse $preview 0x201 ($vx+10) ($vy+30)
            Mouse $preview 0x202 ($vx+130) ($vy+30)
            Save-Png $preview ('layer-created-'+$colorIndex+'.png') | Out-Null
        }
        $blue=[Drawing.Color]::FromArgb(0,120,255)
        $green=[Drawing.Color]::FromArgb(0,160,80)
        Assert-LayerPixel (Join-Path $runDirectory 'layer-created-4.png') $blue
        foreach($id in @(625,626)) {
            if([SnapZyTests.Native]::IsWindowEnabled([SnapZyTests.Native]::GetDlgItem($preview,$id))){throw 'Top layer upward command should be disabled.'}
        }
        Click $preview 628
        $back=Save-Png $preview 'layer-back.png'
        Assert-LayerPixel $back $green
        foreach($id in @(627,628)) {
            if([SnapZyTests.Native]::IsWindowEnabled([SnapZyTests.Native]::GetDlgItem($preview,$id))){throw 'Bottom layer downward command should be disabled.'}
        }
        Click $preview 626
        $forward=Save-Png $preview 'layer-forward.png'
        Assert-LayerPixel $forward $green
        foreach($id in @(625,626,627,628)) {
            if(-not [SnapZyTests.Native]::IsWindowEnabled([SnapZyTests.Native]::GetDlgItem($preview,$id))){throw 'Middle layer should allow all ordering commands.'}
        }
        Click $preview 625
        $front=Save-Png $preview 'layer-front.png'
        Assert-LayerPixel $front $blue
        Click $preview 627
        $backward=Save-Png $preview 'layer-backward.png'
        Assert-LayerPixel $backward $green
        Click $preview 607
        $undo=Save-Png $preview 'layer-undo.png'
        if((Get-FileHash $front).Hash -ne (Get-FileHash $undo).Hash){throw 'Layer Undo did not restore exact pixels.'}
        Click $preview 608
        $redo=Save-Png $preview 'layer-redo.png'
        if((Get-FileHash $backward).Hash -ne (Get-FileHash $redo).Hash){throw 'Layer Redo did not restore exact pixels.'}
        # A layer command cancels an uncommitted move instead of moving both objects.
        Click $preview 601
        Mouse $preview 0x201 ($vx+60) ($vy+30)
        Mouse $preview 0x200 ($vx+60) ($vy+50)
        Click $preview 628
        Mouse $preview 0x202 ($vx+60) ($vy+50)
        $cancelMove=Save-Png $preview 'layer-cancel-move.png'
        Assert-LayerPixel $cancelMove $blue
        $image=[Drawing.Bitmap]::FromFile($cancelMove)
        try {if($image.GetPixel(60,50).ToArgb() -ne $fixture.BackColor.ToArgb()){throw 'Layer command committed the unfinished move.'}}finally{$image.Dispose()}
        Click $preview 201
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -eq 'Copied'} 'Layer clipboard copy failed.' | Out-Null
        $image=[Drawing.Bitmap]::FromFile($cancelMove)
        $copied=[Windows.Forms.Clipboard]::GetImage()
        if(-not $copied){throw 'Layer clipboard image is missing.'}
        $bitmap=[Drawing.Bitmap]::new($copied)
        try {
            if($bitmap.Width -ne 160 -or $bitmap.Height -ne 80){throw 'Layer clipboard dimensions are wrong.'}
            for($y=0;$y -lt 80;$y++){for($x=0;$x -lt 160;$x++){
                if($image.GetPixel($x,$y).ToArgb() -ne $bitmap.GetPixel($x,$y).ToArgb()){throw 'Layer PNG and clipboard pixels disagree.'}
            }}
        }finally{$image.Dispose();$bitmap.Dispose();$copied.Dispose()}
        Snapshot-Editor $preview 'layers-en.png'
        Click $preview 615
        $help=Wait-For {Window-Of '#32770'} 'Layer Help did not open.'
        $message=Wait-For {
            $text=([SnapZyTests.Native]::Children($help) | ForEach-Object {[SnapZyTests.Native]::Text($_)}) -join ' '
            if($text.Contains('Bring to front') -and $text.Contains('Blur affects only objects below')){return $text}
        } 'Layer or ordered blur Help is missing.'
        Dismiss-Message $help 1
        Click $preview 614
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,614)) -eq 'TH'} 'Layer language did not switch.' | Out-Null
        if([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,625)) -ne $thaiFront){throw 'Layer accessible label did not switch to Thai.'}
        Snapshot-Editor $preview 'layers-th.png'
        Click $preview 615
        $help=Wait-For {Window-Of '#32770'} 'Thai layer Help did not open.'
        $message=Wait-For {
            $text=([SnapZyTests.Native]::Children($help) | ForEach-Object {[SnapZyTests.Native]::Text($_)}) -join ' '
            if($text.Contains($thaiFront) -and $text.Contains($thaiBlur)){return $text}
        } 'Thai layer Help is missing.'
        Dismiss-Message $help 1
        Click $preview 614
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,625)) -eq 'Bring to front'} 'Layer accessible label did not switch back.' | Out-Null
        for($i=0;$i -lt 64;$i++){Click $preview 607}
        $reset=Save-Png $preview 'layers-reset.png'
        if((Get-FileHash $png).Hash -ne (Get-FileHash $reset).Hash){throw 'Layer history reset did not restore source.'}
        Click $preview 604
        Set-LayerTestColor $preview 0
        Set-Parameter $preview 5
        $measurements.layers='PASS'
    }
    if ($CalloutChecks) {
        foreach($id in @(620,621,622,624)) {if([SnapZyTests.Native]::GetDlgItem($preview,$id) -eq [IntPtr]::Zero){throw "Text background/callout control $id is missing."}}
        $r=[SnapZyTests.Native]::Client($preview)
        $vx=[int](($r.Right-$r.Left-160)/2);$vy=145+[int](($r.Bottom-$r.Top-175-80)/2)
        Click $preview 624
        if([SnapZyTests.Native]::SendMessage([SnapZyTests.Native]::GetDlgItem($preview,620),0xf0,[IntPtr]::Zero,[IntPtr]::Zero).ToInt32() -ne 0){throw 'Callout default background is not transparent.'}
        Set-Parameter $preview 12
        Mouse $preview 0x201 ($vx+20) ($vy+12)
        $entry=Wait-For {Window-Of 'SnapZyRustPreview.Text'} 'Callout text entry did not open.'
        [SnapZyTests.Native]::WriteText([SnapZyTests.Native]::GetDlgItem($entry,701),0x000c,[IntPtr]::Zero,'Text') | Out-Null
        Click $entry 1
        Wait-For {-not (Window-Of 'SnapZyRustPreview.Text')} 'Callout text did not apply.' | Out-Null
        $transparent=Save-Png $preview 'callout-transparent.png'
        [SnapZyTests.Native]::SendMessage([SnapZyTests.Native]::GetDlgItem($preview,620),0xf1,[IntPtr](1),[IntPtr]::Zero) | Out-Null
        Click $preview 620
        $filled=Save-Png $preview 'callout-filled.png'
        $image=[Drawing.Bitmap]::FromFile($filled)
        try {if($image.GetPixel(21,12).ToArgb() -ne [Drawing.Color]::White.ToArgb()){throw 'Callout white fill did not apply independently of red text.'}}finally{$image.Dispose()}
        Click $preview 607
        $undone=Save-Png $preview 'callout-fill-undo.png'
        if((Get-FileHash $undone).Hash -ne (Get-FileHash $transparent).Hash){throw 'Fill Undo did not restore transparency.'}
        Click $preview 608
        $redone=Save-Png $preview 'callout-fill-redo.png'
        if((Get-FileHash $redone).Hash -ne (Get-FileHash $filled).Hash){throw 'Fill Redo differs.'}
        Click $preview 601
        Mouse $preview 0x201 ($vx+25) ($vy+20)
        Mouse $preview 0x202 ($vx+25) ($vy+20)
        Snapshot-Editor $preview 'callout-handles.png'
        Click $preview 201
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -eq 'Copied'} 'Callout clipboard copy failed.' | Out-Null
        $a=[Drawing.Bitmap]::FromFile($filled);$copy=[Windows.Forms.Clipboard]::GetImage();$b=[Drawing.Bitmap]::new($copy)
        try {for($y=0;$y -lt 80;$y++){for($x=0;$x -lt 160;$x++){if($a.GetPixel($x,$y).ToArgb() -ne $b.GetPixel($x,$y).ToArgb()){throw 'Callout PNG and clipboard differ.'}}}}finally{$a.Dispose();$b.Dispose();$copy.Dispose()}
        [SnapZyTests.Native]::WriteText([SnapZyTests.Native]::GetDlgItem($preview,622),0x000c,[IntPtr]::Zero,'9') | Out-Null
        Click $preview 622
        $thick=Save-Png $preview 'callout-border-thick.png'
        if((Get-FileHash $thick).Hash -eq (Get-FileHash $filled).Hash){throw 'Bubble border width did not change.'}
        if([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,611)) -ne '12'){throw 'Border changed text font size.'}
        Click $preview 607
        $borderUndo=Save-Png $preview 'callout-border-undo.png'
        if((Get-FileHash $borderUndo).Hash -ne (Get-FileHash $filled).Hash){throw 'Border Undo changed pixels.'}
        if([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,622)) -ne '5'){throw 'Border field is stale after Undo.'}
        $handles=[Drawing.Bitmap]::FromFile((Join-Path $runDirectory 'callout-handles.png'))
        try {
            if($handles.GetPixel($vx+20,$vy+12).ToArgb() -ne [Drawing.Color]::White.ToArgb()){throw 'Bubble resize handle missing.'}
            $xs=@();$ys=@()
            for($y=$vy+50;$y -lt $vy+80;$y++){for($x=$vx+10;$x -lt $vx+70;$x++){
                if($handles.GetPixel($x,$y).ToArgb() -eq [Drawing.Color]::FromArgb(22,103,218).ToArgb()){$xs+=$x;$ys+=$y}
            }}
            if(-not $xs.Count){throw 'Bubble tail handle missing.'}
            $tx=[int]((($xs|Measure-Object -Minimum).Minimum+($xs|Measure-Object -Maximum).Maximum)/2)
            $ty=[int]((($ys|Measure-Object -Minimum).Minimum+($ys|Measure-Object -Maximum).Maximum)/2)
        }finally{$handles.Dispose()}
        $parameter=([int64]($ty-band 0xffff)-shl 16)-bor ($tx-band 0xffff)
        [SnapZyTests.Native]::SendMessage($preview,0x203,[IntPtr](1),[IntPtr]$parameter) | Out-Null
        if(Window-Of 'SnapZyRustPreview.Text'){throw 'Tail double-click opened text entry instead of dragging.'}
        Mouse $preview 0x202 $tx $ty
        $still=Save-Png $preview 'callout-tail-no-motion.png'
        if((Get-FileHash $still).Hash -ne (Get-FileHash $filled).Hash){throw 'Tail click without movement changed image.'}
        [SnapZyTests.Native]::SendMessage($preview,0x201,[IntPtr](1),[IntPtr]$parameter) | Out-Null
        $draftPoint=([int64](($vy+50)-band 0xffff)-shl 16)-bor (($vx+5)-band 0xffff)
        [SnapZyTests.Native]::SendMessage($preview,0x200,[IntPtr](1),[IntPtr]$draftPoint) | Out-Null
        Snapshot-Editor $preview 'callout-tail-draft.png'
        $a=[Drawing.Bitmap]::FromFile((Join-Path $runDirectory 'callout-handles.png'))
        $b=[Drawing.Bitmap]::FromFile((Join-Path $runDirectory 'callout-tail-draft.png'))
        try {for($y=$vy+17;$y -lt $vy+27;$y++){for($x=$vx+25;$x -lt $vx+41;$x++){
            if($a.GetPixel($x,$y).ToArgb() -ne $b.GetPixel($x,$y).ToArgb()){throw 'Tail draft obscured cached text.'}
        }}}finally{$a.Dispose();$b.Dispose()}
        Mouse $preview 0x202 ($vx+5) ($vy+50)
        $tail=Save-Png $preview 'callout-tail-left.png'
        if((Get-FileHash $tail).Hash -eq (Get-FileHash $filled).Hash){throw 'Tail drag did not change bubble.'}
        Click $preview 607
        $tailUndo=Save-Png $preview 'callout-tail-undo.png'
        if((Get-FileHash $tailUndo).Hash -ne (Get-FileHash $filled).Hash){throw 'Tail drag was not one reversible edit.'}
        Click $preview 608
        $tailRedo=Save-Png $preview 'callout-tail-redo.png'
        if((Get-FileHash $tailRedo).Hash -ne (Get-FileHash $tail).Hash){throw 'Tail Redo differs.'}
        Click $preview 607
        foreach($cancel in @(0x100,0x215)) {
            Mouse $preview 0x201 $tx $ty
            Mouse $preview 0x200 ($vx+5) ($vy+50)
            [SnapZyTests.Native]::PostMessage($preview,$cancel,[IntPtr](27),[IntPtr]::Zero) | Out-Null
            Mouse $preview 0x202 $tx $ty
            $cancelled=Save-Png $preview ('callout-tail-cancel-'+$cancel+'.png')
            if((Get-FileHash $cancelled).Hash -ne (Get-FileHash $filled).Hash){throw 'Tail cancellation changed bubble.'}
        }
        Click $preview 621
        $menu=Wait-For {[SnapZyTests.Native]::Windows($process.Id,'#32768') | Where-Object {[SnapZyTests.Native]::IsWindowVisible($_)} | Select-Object -First 1} 'Background palette missing.'
        for($i=0;$i -lt 10 -and [SnapZyTests.Native]::HighlightedMenuItem($menu) -ne 3;$i++) {
            $old=[SnapZyTests.Native]::HighlightedMenuItem($menu)
            [SnapZyTests.Native]::PostMessage($preview,0x100,[IntPtr](40),[IntPtr]::Zero) | Out-Null
            Wait-For {[SnapZyTests.Native]::HighlightedMenuItem($menu) -ge 0 -and [SnapZyTests.Native]::HighlightedMenuItem($menu) -ne $old} 'Background palette navigation failed.' | Out-Null
        }
        [SnapZyTests.Native]::PostMessage($preview,0x100,[IntPtr](13),[IntPtr]::Zero) | Out-Null
        Wait-For {-not [SnapZyTests.Native]::IsWindowVisible($menu)} 'Background palette did not close.' | Out-Null
        $green=Save-Png $preview 'callout-green.png'
        $image=[Drawing.Bitmap]::FromFile($green)
        try {if($image.GetPixel(21,12).ToArgb() -ne [Drawing.Color]::FromArgb(0,160,80).ToArgb()){throw 'Independent background color did not apply.'}}finally{$image.Dispose()}
        [SnapZyTests.Native]::SendMessage([SnapZyTests.Native]::GetDlgItem($preview,620),0xf1,[IntPtr](0),[IntPtr]::Zero) | Out-Null
        Click $preview 620
        $off=Save-Png $preview 'callout-background-off.png'
        if((Get-FileHash $off).Hash -ne (Get-FileHash $transparent).Hash){throw 'Disabling fill changed border or text.'}
        [SnapZyTests.Native]::SendMessage([SnapZyTests.Native]::GetDlgItem($preview,620),0xf1,[IntPtr](1),[IntPtr]::Zero) | Out-Null
        Click $preview 620
        $on=Save-Png $preview 'callout-background-restored.png'
        if((Get-FileHash $on).Hash -ne (Get-FileHash $green).Hash){throw 'Background color was not remembered.'}
        $parameter=([int64](($vy+23)-band 0xffff)-shl 16)-bor (($vx+33)-band 0xffff)
        [SnapZyTests.Native]::PostMessage($preview,0x203,[IntPtr](1),[IntPtr]$parameter) | Out-Null
        $entry=Wait-For {Window-Of 'SnapZyRustPreview.Text'} 'Bubble did not reopen for editing.'
        [SnapZyTests.Native]::WriteText([SnapZyTests.Native]::GetDlgItem($entry,701),0x000c,[IntPtr]::Zero,"$thaiText`r`nEnglish") | Out-Null
        Click $entry 1
        Wait-For {-not (Window-Of 'SnapZyRustPreview.Text')} 'Edited bubble did not close.' | Out-Null
        $edited=Save-Png $preview 'callout-thai-english.png'
        if((Get-FileHash $edited).Hash -eq (Get-FileHash $green).Hash){throw 'Bubble text edit did not change pixels.'}
        Click $preview 607
        $editUndo=Save-Png $preview 'callout-text-undo.png'
        if((Get-FileHash $editUndo).Hash -ne (Get-FileHash $green).Hash){throw 'Text edit lost bubble decorations.'}
        for($i=0;$i -lt 64;$i++){Click $preview 607}
        Save-Png $preview 'callout-before-plain-reset.png' | Out-Null
        [SnapZyTests.Native]::SendMessage($preview,0x0111,[IntPtr](617),[SnapZyTests.Native]::GetDlgItem($preview,617)) | Out-Null
        [SnapZyTests.Native]::SendMessage([SnapZyTests.Native]::GetDlgItem($preview,620),0xf1,[IntPtr](0),[IntPtr]::Zero) | Out-Null
        Click $preview 620
        Mouse $preview 0x201 ($vx+20) ($vy+12)
        $entry=Wait-For {Window-Of 'SnapZyRustPreview.Text'} 'Plain text entry missing.'
        [SnapZyTests.Native]::WriteText([SnapZyTests.Native]::GetDlgItem($entry,701),0x000c,[IntPtr]::Zero,'Hi') | Out-Null
        Click $entry 1
        Wait-For {-not (Window-Of 'SnapZyRustPreview.Text')} 'Plain text did not apply.' | Out-Null
        $plain=Save-Png $preview 'text-transparent.png'
        [SnapZyTests.Native]::SendMessage([SnapZyTests.Native]::GetDlgItem($preview,620),0xf1,[IntPtr](1),[IntPtr]::Zero) | Out-Null
        Click $preview 620
        $plainFill=Save-Png $preview 'text-green-background.png'
        $image=[Drawing.Bitmap]::FromFile($plainFill)
        try {if($image.GetPixel(20,12).ToArgb() -ne [Drawing.Color]::FromArgb(0,160,80).ToArgb()){throw 'Plain text did not reuse independent background color.'}}finally{$image.Dispose()}
        Click $preview 607
        $plainUndo=Save-Png $preview 'text-background-undo.png'
        if((Get-FileHash $plainUndo).Hash -ne (Get-FileHash $plain).Hash){throw 'Plain text fill Undo differs.'}
        for($i=0;$i -lt 64;$i++){Click $preview 607}
        [SnapZyTests.Native]::SendMessage([SnapZyTests.Native]::GetDlgItem($preview,620),0xf1,[IntPtr](0),[IntPtr]::Zero) | Out-Null
        Click $preview 620
        Click $preview 601
        $reset=Save-Png $preview 'callout-reset.png'
        if((Get-FileHash $reset).Hash -ne (Get-FileHash $png).Hash){throw 'Callout test reset changed original image.'}
        $measurements.callout='PASS'
    }
    if ($PaintChecks) {
        $paintFailures=@()
        $r=[SnapZyTests.Native]::Client($preview)
        $px=[int](($r.Right-$r.Left-160)/2)
        $py=145+[int](($r.Bottom-$r.Top-175-80)/2)
        foreach($tool in @(603,604,605,606)) {
        if($tool -eq 606){Select-TestShape $preview $tool}else{Click $preview $tool}
        Start-Sleep -Milliseconds 100
        Mouse $preview 0x201 ($px+20) ($py+20)
        if($tool -eq 603){Mouse $preview 0x200 ($px+50) ($py+34)}
        Mouse $preview 0x200 ($px+100) ($py+60)
        Start-Sleep -Milliseconds 100
        Snapshot-Editor $preview "draft-$tool.png"
        Mouse $preview 0x202 ($px+100) ($py+60)
        Start-Sleep -Milliseconds 100
        Snapshot-Editor $preview "finished-$tool.png"
        $draft=[Drawing.Bitmap]::FromFile((Join-Path $runDirectory "draft-$tool.png"))
        $done=[Drawing.Bitmap]::FromFile((Join-Path $runDirectory "finished-$tool.png"))
        try {
            $different=0;$soft=0
            for($y=22;$y -lt 59;$y++){for($x=22;$x -lt 98;$x++){
                # A newly completed line/arrow now gains its midpoint edit handle.
                if($tool -in @(604,605) -and [Math]::Abs($x-60) -le 5 -and [Math]::Abs($y-40) -le 5){continue}
                $a=$draft.GetPixel($px+$x,$py+$y);$b=$done.GetPixel($px+$x,$py+$y)
                if($a.ToArgb() -ne $b.ToArgb()){$different++}
                if($a.ToArgb() -ne $fixture.BackColor.ToArgb() -and $a.ToArgb() -ne [Drawing.Color]::Red.ToArgb()){$soft++}
            }}
            if($different -ne 0 -or $soft -eq 0){$paintFailures+="Tool $tool changed after release or has no antialiasing: changed=$different; antialiased=$soft"}
        }finally{$draft.Dispose();$done.Dispose()}
        Click $preview 607
        Start-Sleep -Milliseconds 100
        }
        # Synchronous idle moves must not leave repaint work queued.
        $repaints=0
        for($i=0;$i -lt 20;$i++){
            [SnapZyTests.Native]::UpdateWindow($preview) | Out-Null
            $parameter=([int64]($py+10)-shl 16)-bor ($px+10+$i)
            [SnapZyTests.Native]::SendMessage($preview,0x200,[IntPtr]::Zero,[IntPtr]$parameter) | Out-Null
            $dirty=[SnapZyTests.Native+Rect]::new()
            if([SnapZyTests.Native]::GetUpdateRect($preview,[ref]$dirty,$false)){$repaints++}
        }
        if($repaints -ne 0){$paintFailures+="Idle mouse movement queued $repaints unnecessary editor repaints."}
        if($paintFailures.Count){throw ($paintFailures -join ' ')}
        $before=[SnapZyTests.Native]::GetGuiResources($process.Handle,0)
        for($i=0;$i -lt 30;$i++) {
            $w=if($i%2){816}else{700};$h=if($i%2){619}else{450}
            [SnapZyTests.Native]::SetWindowPos($preview,[IntPtr]::Zero,0,0,$w,$h,6) | Out-Null
            [SnapZyTests.Native]::UpdateWindow($preview) | Out-Null
        }
        $after=[SnapZyTests.Native]::GetGuiResources($process.Handle,0)
        if($after -gt $before+2){throw "Editor resize leaked drawing resources: before=$before; after=$after"}
        $measurements.resize_gdi_delta=[int]$after-[int]$before
        $measurements.paint='PASS'
    }
    if ($AnnotationChecks) {
        foreach($id in @(617,618,619)) {if([SnapZyTests.Native]::GetDlgItem($preview,$id) -eq [IntPtr]::Zero){throw "Annotation tool $id is missing."}}
        $r=[SnapZyTests.Native]::Client($preview)
        $vx=[int](($r.Right-$r.Left-160)/2)
        $vy=145+[int](($r.Bottom-$r.Top-175-80)/2)
        Click $preview 617
        Mouse $preview 0x201 ($vx+2) ($vy+1)
        $entry=Wait-For {Window-Of 'SnapZyRustPreview.Text'} 'Native multiline text dialog did not open.'
        $input=[SnapZyTests.Native]::GetDlgItem($entry,701)
        [SnapZyTests.Native]::WriteText($input,0x000c,[IntPtr]::Zero,"$thaiText`r`nEnglish") | Out-Null
        Click $entry 1
        Wait-For {-not (Window-Of 'SnapZyRustPreview.Text')} 'Text confirmation did not close dialog.' | Out-Null
        $textPng=Save-Png $preview 'thai-text.png'
        $textImage=[Drawing.Bitmap]::FromFile($textPng)
        try {
            $changed=0
            for($y=2;$y -lt 65;$y++){for($x=2;$x -lt 140;$x++){if($textImage.GetPixel($x,$y).ToArgb() -ne $fixture.BackColor.ToArgb()){$changed++}}}
            if($changed -lt 100){throw 'Thai/English text was not rendered.'}
        }finally{$textImage.Dispose()}
        Mouse $preview 0x201 ($vx+15) ($vy+15)
        $entry=Wait-For {Window-Of 'SnapZyRustPreview.Text'} 'Text tool did not reopen existing text.'
        if(-not [SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($entry,701)).Contains($thaiText)){throw 'Text tool created a blank duplicate instead of editing existing text.'}
        Click $entry 2
        Wait-For {-not (Window-Of 'SnapZyRustPreview.Text')} 'Text tool cancel did not close.' | Out-Null
        Click $preview 601
        Mouse $preview 0x203 ($vx+15) ($vy+15)
        $entry=Wait-For {Window-Of 'SnapZyRustPreview.Text'} 'Double-click text editing did not open.'
        [SnapZyTests.Native]::WriteText([SnapZyTests.Native]::GetDlgItem($entry,701),0x000c,[IntPtr]::Zero,'Edited') | Out-Null
        Click $entry 2
        Wait-For {-not (Window-Of 'SnapZyRustPreview.Text')} 'Text cancellation did not close.' | Out-Null
        $cancelPng=Save-Png $preview 'thai-cancelled.png'
        if((Get-FileHash $textPng).Hash -ne (Get-FileHash $cancelPng).Hash){throw 'Cancelled text edit changed exported image.'}
        Mouse $preview 0x201 ($vx+15) ($vy+15)
        Mouse $preview 0x202 ($vx+15) ($vy+15)
        Set-Parameter $preview 32
        $sizedText=Save-Png $preview 'text-sized.png'
        if((Get-FileHash $textPng).Hash -eq (Get-FileHash $sizedText).Hash){throw 'Font size was not applied.'}
        Click $preview 607
        $unsized=Save-Png $preview 'text-size-undo.png'
        if((Get-FileHash $textPng).Hash -ne (Get-FileHash $unsized).Hash){throw 'Font size Undo did not restore the mask.'}
        Mouse $preview 0x203 ($vx+15) ($vy+15)
        $entry=Wait-For {Window-Of 'SnapZyRustPreview.Text'} 'Text re-edit did not open.'
        [SnapZyTests.Native]::WriteText([SnapZyTests.Native]::GetDlgItem($entry,701),0x000c,[IntPtr]::Zero,'Edited') | Out-Null
        Click $entry 1
        Wait-For {-not (Window-Of 'SnapZyRustPreview.Text')} 'Text edit did not close.' | Out-Null
        $editedText=Save-Png $preview 'text-edited.png'
        if((Get-FileHash $textPng).Hash -eq (Get-FileHash $editedText).Hash){throw 'Confirmed text edit did not change image.'}
        Click $preview 607
        $undoText=Save-Png $preview 'text-undo.png'
        if((Get-FileHash $textPng).Hash -ne (Get-FileHash $undoText).Hash){throw 'Text edit Undo did not restore text.'}
        Click $preview 618
        foreach($x in @(120,145)){Mouse $preview 0x201 ($vx+$x) ($vy+50);Mouse $preview 0x202 ($vx+$x) ($vy+50)}
        $steps=Save-Png $preview 'steps.png'
        Click $preview 609
        Mouse $preview 0x201 ($vx+145) ($vy+50);Mouse $preview 0x202 ($vx+145) ($vy+50)
        $resumed=Save-Png $preview 'steps-resumed.png'
        if((Get-FileHash $steps).Hash -ne (Get-FileHash $resumed).Hash){throw 'Deleting highest step did not resume same number.'}
        Set-Parameter $preview 48
        $sizedStep=Save-Png $preview 'step-sized.png'
        if((Get-FileHash $resumed).Hash -eq (Get-FileHash $sizedStep).Hash){throw 'Circle diameter did not apply.'}
        Click $preview 607
        $unsizedStep=Save-Png $preview 'step-size-undo.png'
        if((Get-FileHash $resumed).Hash -ne (Get-FileHash $unsizedStep).Hash){throw 'Circle diameter Undo did not restore.'}
        Click $preview 619
        Mouse $preview 0x201 ($vx+2) ($vy+2);Mouse $preview 0x200 ($vx+110) ($vy+65)
        Snapshot-Editor $preview 'blur-region-draft.png'
        Mouse $preview 0x202 ($vx+110) ($vy+65)
        $blurred=Save-Png $preview 'blurred.png'
        if((Get-FileHash $resumed).Hash -eq (Get-FileHash $blurred).Hash){throw 'Blur did not change composition beneath it.'}
        Set-Parameter $preview 14
        $stronger=Save-Png $preview 'blur-stronger.png'
        if((Get-FileHash $blurred).Hash -eq (Get-FileHash $stronger).Hash){throw 'Blur strength did not apply.'}
        Click $preview 607
        $originalBlur=Save-Png $preview 'blur-strength-undo.png'
        if((Get-FileHash $blurred).Hash -ne (Get-FileHash $originalBlur).Hash){throw 'Blur strength Undo did not restore.'}
        Click $preview 201
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -eq 'Copied'} 'Annotated clipboard copy failed.' | Out-Null
        $image=[Drawing.Bitmap]::FromFile($blurred)
        $copied=[Windows.Forms.Clipboard]::GetImage()
        if(-not $copied){throw 'Annotated clipboard image missing.'}
        $bitmap=[Drawing.Bitmap]::new($copied)
        try {for($y=0;$y -lt 80;$y++){for($x=0;$x -lt 160;$x++){if($image.GetPixel($x,$y).ToArgb() -ne $bitmap.GetPixel($x,$y).ToArgb()){throw 'Clipboard and PNG disagree on text/steps/blur.'}}}}finally{$image.Dispose();$bitmap.Dispose();$copied.Dispose()}
        Click $preview 609
        $restored=Save-Png $preview 'blur-deleted.png'
        if((Get-FileHash $resumed).Hash -ne (Get-FileHash $restored).Hash){throw 'Deleting blur did not restore original text/steps.'}
        Click $preview 614
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,614)) -eq 'TH'} 'Annotation language did not switch.' | Out-Null
        if(-not [SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,616)).Contains($thaiBlur)){throw 'Tool-specific size label was not localized.'}
        Click $preview 614
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,614)) -eq 'EN'} 'Annotation language did not switch back.' | Out-Null
        Snapshot-Editor $preview 'annotation-tools.png'
        # Return to the initial image for existing editor regression checks.
        for($i=0;$i -lt 64;$i++){Click $preview 607}
        Save-Png $preview 'annotation-reset.png' | Out-Null
        $measurements.annotations='PASS'
    }
    if ($ResizeChecks) {
        $r=[SnapZyTests.Native]::Client($preview)
        $vx=[int](($r.Right-$r.Left-160)/2)
        $vy=145+[int](($r.Bottom-$r.Top-175-80)/2)
        Click $preview 617
        Set-Parameter $preview 12
        Mouse $preview 0x201 ($vx+12) ($vy+12)
        $textDialog=Wait-For {Window-Of 'SnapZyRustPreview.Text'} 'Resize text dialog missing.'
        [SnapZyTests.Native]::WriteText([SnapZyTests.Native]::GetDlgItem($textDialog,701),0x000c,[IntPtr]::Zero,'one two three four') | Out-Null
        Click $textDialog 1
        Wait-For {-not (Window-Of 'SnapZyRustPreview.Text')} 'Resize text dialog did not close.' | Out-Null
        $beforeResize=Save-Png $preview 'resize-original.png'
        Click $preview 601
        Mouse $preview 0x201 ($vx+24) ($vy+20)
        Mouse $preview 0x202 ($vx+24) ($vy+20)
        function Read-TextFrame([string]$Name) {
            Snapshot-Editor $preview $Name
            $b=[Drawing.Bitmap]::FromFile((Join-Path $runDirectory $Name))
            try {
                if($b.GetPixel($vx+12,$vy+12).ToArgb() -ne [Drawing.Color]::White.ToArgb()){throw 'Selected text corner resize handle is not visible.'}
                $xs=@();$ys=@()
                for($y=$vy+6;$y -lt $vy+80;$y++){for($x=$vx+6;$x -lt $vx+160;$x++){
                    if($b.GetPixel($x,$y).ToArgb() -eq [Drawing.Color]::FromArgb(22,103,218).ToArgb()){$xs+=$x;$ys+=$y}
                }}
                if(-not $xs.Count){throw 'Text frame border missing.'}
                return @{Left=($xs|Measure-Object -Minimum).Minimum+4;Top=($ys|Measure-Object -Minimum).Minimum+4;Right=($xs|Measure-Object -Maximum).Maximum-4;Bottom=($ys|Measure-Object -Maximum).Maximum-4}
            }finally{$b.Dispose()}
        }
        $frame=Read-TextFrame 'text-handles-original.png'
        $mid=[int](($frame.Top+$frame.Bottom)/2)
        Mouse $preview 0x201 $frame.Right $mid
        Mouse $preview 0x200 ($vx+72) $mid
        Mouse $preview 0x202 ($vx+72) $mid
        $wrapped=Save-Png $preview 'text-side-wrapped.png'
        if([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,611)) -ne '12'){throw 'Side resize changed font size.'}
        $frame=Read-TextFrame 'text-handles-wrapped.png'
        if($frame.Right-$frame.Left -ne 60){throw 'Text frame did not retain requested 60px width.'}
        $a=[Drawing.Bitmap]::FromFile($beforeResize);$b=[Drawing.Bitmap]::FromFile($wrapped)
        try {
            $aRows=@();$bRows=@()
            for($y=0;$y -lt 80;$y++){for($x=0;$x -lt 160;$x++){
                if($a.GetPixel($x,$y).ToArgb() -ne $fixture.BackColor.ToArgb()){$aRows+=$y}
                if($b.GetPixel($x,$y).ToArgb() -ne $fixture.BackColor.ToArgb()){$bRows+=$y}
            }}
            if(($bRows|Measure-Object -Maximum).Maximum -le ($aRows|Measure-Object -Maximum).Maximum){throw 'Side resize did not reflow words onto more lines.'}
        }finally{$a.Dispose();$b.Dispose()}
        Mouse $preview 0x201 $frame.Right $frame.Bottom
        Mouse $preview 0x200 ([int](2*$frame.Right-$frame.Left)) ([int](2*$frame.Bottom-$frame.Top))
        Mouse $preview 0x202 ([int](2*$frame.Right-$frame.Left)) ([int](2*$frame.Bottom-$frame.Top))
        $scaled=Save-Png $preview 'text-corner-scaled.png'
        if([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,611)) -ne '24'){throw 'Corner resize did not double font size.'}
        Click $preview 201
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -eq 'Copied'} 'Resized clipboard copy failed.' | Out-Null
        $a=[Drawing.Bitmap]::FromFile($scaled);$copied=[Windows.Forms.Clipboard]::GetImage();$b=[Drawing.Bitmap]::new($copied)
        try {for($y=0;$y -lt 80;$y++){for($x=0;$x -lt 160;$x++){if($a.GetPixel($x,$y).ToArgb() -ne $b.GetPixel($x,$y).ToArgb()){throw 'Resized PNG/clipboard pixels differ.'}}}}finally{$a.Dispose();$b.Dispose();$copied.Dispose()}
        Click $preview 607
        $undo=Save-Png $preview 'text-resize-undo.png'
        if((Get-FileHash $undo).Hash -ne (Get-FileHash $wrapped).Hash){throw 'Corner resize was not one reversible edit.'}
        Click $preview 608
        $redo=Save-Png $preview 'text-resize-redo.png'
        if((Get-FileHash $redo).Hash -ne (Get-FileHash $scaled).Hash){throw 'Corner resize Redo did not restore pixels.'}
        Click $preview 607
        $parameter=([int64]([int]$frame.Bottom-band 0xffff)-shl 16)-bor ([int]$frame.Right-band 0xffff)
        [SnapZyTests.Native]::SendMessage($preview,0x203,[IntPtr](1),[IntPtr]$parameter) | Out-Null
        if(Window-Of 'SnapZyRustPreview.Text'){throw 'Double-click on a resize handle opened text entry instead of preserving handle priority.'}
        Mouse $preview 0x202 $frame.Right $frame.Bottom
        foreach($cancel in @(0x100,0x215)) {
            Mouse $preview 0x201 $frame.Right $frame.Bottom
            Mouse $preview 0x200 ([int](2*$frame.Right-$frame.Left)) ([int](2*$frame.Bottom-$frame.Top))
            [SnapZyTests.Native]::PostMessage($preview,$cancel,[IntPtr](27),[IntPtr]::Zero) | Out-Null
            Mouse $preview 0x202 $frame.Right $frame.Bottom
            $cancelled=Save-Png $preview ('text-resize-cancel-'+$cancel+'.png')
            if((Get-FileHash $cancelled).Hash -ne (Get-FileHash $wrapped).Hash){throw 'Resize cancellation mutated text.'}
        }
        Click $preview 607
        $undo=Save-Png $preview 'text-reflow-undo.png'
        if((Get-FileHash $undo).Hash -ne (Get-FileHash $beforeResize).Hash){throw 'Side resize was not one reversible edit.'}
        for($i=0;$i -lt 64;$i++){Click $preview 607}
        Save-Png $preview 'text-resize-reset.png' | Out-Null
        Click $preview 617
        Set-Parameter $preview 16
        Mouse $preview 0x201 ($vx+12) ($vy+12)
        $textDialog=Wait-For {Window-Of 'SnapZyRustPreview.Text'} 'Thai resize text dialog missing.'
        $thaiInput=$thaiText+' English'+[Environment]::NewLine+$thaiText
        [SnapZyTests.Native]::WriteText([SnapZyTests.Native]::GetDlgItem($textDialog,701),0x000c,[IntPtr]::Zero,$thaiInput) | Out-Null
        Click $textDialog 1
        Wait-For {-not (Window-Of 'SnapZyRustPreview.Text')} 'Thai resize dialog did not close.' | Out-Null
        Save-Png $preview 'text-thai-original.png' | Out-Null
        Click $preview 601
        Mouse $preview 0x201 ($vx+24) ($vy+24)
        Mouse $preview 0x202 ($vx+24) ($vy+24)
        $frame=Read-TextFrame 'text-thai-handles.png'
        $mid=[int](($frame.Top+$frame.Bottom)/2)
        Mouse $preview 0x201 $frame.Right $mid
        Mouse $preview 0x200 ($vx+100) $mid
        Mouse $preview 0x202 ($vx+100) $mid
        Save-Png $preview 'text-thai-wrapped.png' | Out-Null
        if([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,611)) -ne '16'){throw 'Thai reflow changed font size.'}
        Snapshot-Editor $preview 'text-thai-frame.png'
        Click $preview 617
        Mouse $preview 0x201 ($vx+24) ($vy+24)
        $textDialog=Wait-For {Window-Of 'SnapZyRustPreview.Text'} 'Reflowed Thai text did not reopen.'
        if([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($textDialog,701)).Replace("`r",'') -ne $thaiInput.Replace("`r",'')){throw 'Reflow changed Thai text or explicit line breaks.'}
        Click $textDialog 2
        Wait-For {-not (Window-Of 'SnapZyRustPreview.Text')} 'Thai resize cancel did not close.' | Out-Null
        for($i=0;$i -lt 64;$i++){Click $preview 607}
        Save-Png $preview 'text-thai-reset.png' | Out-Null
        $measurements.text_resize='PASS'
    }
    if ($EditorChecks) {
        $tips=Wait-For {[SnapZyTests.Native]::Windows($process.Id,'tooltips_class32') | Select-Object -First 1} 'Editor tooltips were not created.'
        $toolCount=[SnapZyTests.Native]::SendMessage($tips,0x040d,[IntPtr]::Zero,[IntPtr]::Zero).ToInt64()
        if($toolCount -ne 37) {throw "Editor tooltip count differs: expected=37 actual=$toolCount"}
        if([SnapZyTests.Native]::GetDlgItem($preview,632) -eq [IntPtr]::Zero) {throw 'Editor OCR command is missing.'}
        Click $preview 604
        $r=[SnapZyTests.Native]::Client($preview)
        $vx=[int](($r.Right-$r.Left-160)/2)
        $vy=145+[int](($r.Bottom-$r.Top-175-80)/2)
        Mouse $preview 0x201 ($vx+20) ($vy+20)
        Mouse $preview 0x200 ($vx+80) ($vy+20)
        Mouse $preview 0x202 ($vx+80) ($vy+20)
        Start-Sleep -Milliseconds 150
        Click $preview 607
        Click $preview 608
        Click $preview 601
        Mouse $preview 0x201 ($vx+30) ($vy+20)
        Mouse $preview 0x200 ($vx+30) ($vy+30)
        Mouse $preview 0x202 ($vx+30) ($vy+30)
        Start-Sleep -Milliseconds 150
        Click $preview 202
        $dialog=Wait-For {Window-Of '#32770'} 'Editor PNG dialog did not open.'
        $filename=Wait-For {
            foreach ($c in [SnapZyTests.Native]::Children($dialog)) {
                if ([SnapZyTests.Native]::Class($c) -eq 'Edit' -and [SnapZyTests.Native]::Text($c).Contains('SnapZy-Rust-')) {return $c}
            };return $null
        } 'Editor save filename missing.'
        $edited=Join-Path $runDirectory 'editor.png'
        [SnapZyTests.Native]::WriteText($filename,0x000c,[IntPtr]::Zero,$edited) | Out-Null
        Click $dialog 1
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -eq 'Saved'} 'Editor export failed.' | Out-Null
        $image=[Drawing.Bitmap]::FromFile($edited)
        try {
            if ($image.GetPixel(40,30).ToArgb() -ne [Drawing.Color]::Red.ToArgb()) {throw 'Editor export omitted moved red line.'}
            if ($image.GetPixel(40,20).ToArgb() -ne $fixture.BackColor.ToArgb()) {throw 'Move left the old line in exported pixels.'}
        } finally {$image.Dispose()}
        # Exercise selected-object width and the native color popup.
        $width=[SnapZyTests.Native]::GetDlgItem($preview,611)
        [SnapZyTests.Native]::WriteText($width,0x000c,[IntPtr]::Zero,'2') | Out-Null
        [SnapZyTests.Native]::PostMessage($preview,0x111,[IntPtr](611-bor (512-shl 16)),$width) | Out-Null
        Wait-For {[SnapZyTests.Native]::Text($width) -eq '2'} 'Width did not update.' | Out-Null
        Click $preview 610
        $menu=Wait-For {[SnapZyTests.Native]::Windows($process.Id,'#32768') | Where-Object {[SnapZyTests.Native]::IsWindowVisible($_)} | Select-Object -First 1} 'Color menu did not open.'
        # Native popup Home handling is not reliable; observe actual arrow transitions.
        Wait-For {[SnapZyTests.Native]::HighlightedMenuItem($menu) -ge -1} 'Color menu is not ready.' | Out-Null
        for($i=0;$i -lt 10 -and [SnapZyTests.Native]::HighlightedMenuItem($menu) -ne 3;$i++) {
            $old=[SnapZyTests.Native]::HighlightedMenuItem($menu)
            [SnapZyTests.Native]::PostMessage($preview,0x100,[IntPtr](40),[IntPtr]::Zero) | Out-Null
            Wait-For {$state=[SnapZyTests.Native]::HighlightedMenuItem($menu);$state -ge 0 -and $state -ne $old} 'Color menu selection did not advance.' | Out-Null
        }
        if([SnapZyTests.Native]::HighlightedMenuItem($menu) -ne 3){throw 'Color menu did not select green.'}
        [SnapZyTests.Native]::PostMessage($preview,0x100,[IntPtr](13),[IntPtr]::Zero) | Out-Null
        Wait-For {-not [SnapZyTests.Native]::IsWindowVisible($menu)} 'Color menu did not close.' | Out-Null
        $styled=Save-Png $preview 'styled.png'
        $image=[Drawing.Bitmap]::FromFile($styled)
        try {
            if($image.GetPixel(40,29).ToArgb() -ne [Drawing.Color]::FromArgb(0,160,80).ToArgb()) {throw 'Selected line color did not change to green.'}
            if($image.GetPixel(40,27).ToArgb() -ne $fixture.BackColor.ToArgb()) {throw 'Selected line width did not change to 2px.'}
        }finally{$image.Dispose()}
        Click $preview 609
        $deleted=Save-Png $preview 'deleted.png'
        $image=[Drawing.Bitmap]::FromFile($deleted)
        try {if($image.GetPixel(40,30).ToArgb() -ne $fixture.BackColor.ToArgb()) {throw 'Delete did not remove the selected line.'}}finally{$image.Dispose()}
        Click $preview 607
        # Cancel a pen gesture, then draw pen, arrow and rectangle using real controls.
        Click $preview 603
        Mouse $preview 0x201 ($vx+10) ($vy+60)
        Mouse $preview 0x200 ($vx+50) ($vy+60)
        [SnapZyTests.Native]::PostMessage($preview,0x100,[IntPtr](27),[IntPtr]::Zero) | Out-Null
        $cancelled=Save-Png $preview 'cancelled.png'
        $image=[Drawing.Bitmap]::FromFile($cancelled)
        try {if($image.GetPixel(30,60).ToArgb() -ne $fixture.BackColor.ToArgb()) {throw 'Escape committed an unfinished stroke.'}}finally{$image.Dispose()}
        foreach($tool in @(603,605,606)) {
            if($tool -eq 606){Select-TestShape $preview $tool}else{Click $preview $tool}
            $tx=switch($tool){603 {10}605 {65}606 {115}}
            Mouse $preview 0x201 ($vx+$tx) ($vy+45)
            Mouse $preview 0x200 ($vx+$tx+10) ($vy+50)
            Mouse $preview 0x202 ($vx+$tx+30) ($vy+65)
        }
        $all=Save-Png $preview 'all-tools.png'
        $image=[Drawing.Bitmap]::FromFile($all)
        try {
            foreach($pt in @(@(10,45),@(80,55),@(115,55))) {
                $pixel=$image.GetPixel($pt[0],$pt[1])
                if($pixel.ToArgb() -eq $fixture.BackColor.ToArgb()) {throw "A tool did not appear in PNG at $pt."}
            }
        }finally{$image.Dispose()}
        Click $preview 614
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,614)) -eq 'TH'} 'Editor language did not switch.' | Out-Null
        Snapshot-Editor $preview 'editor-th.png'
        Click $preview 614
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,614)) -eq 'EN'} 'Editor language did not switch back.' | Out-Null
        Click $preview 615
        $help=Wait-For {Window-Of '#32770'} 'Editor help did not open.'
        Dismiss-Message $help 1
        Snapshot-Editor $preview 'editor-en.png'
        $shot=[Drawing.Bitmap]::FromFile((Join-Path $runDirectory 'editor-en.png'))
        try {
            if($shot.GetPixel(30,115).ToArgb() -ne [Drawing.Color]::FromArgb(0,160,80).ToArgb()) {throw 'Editor toolbar disappeared after parent repaint.'}
        }finally{$shot.Dispose()}
        $measurements.editor_dpi=[SnapZyTests.Native]::GetDpiForWindow($preview)
        Click $preview 602
        Mouse $preview 0x201 ($vx+100) ($vy+70)
        Mouse $preview 0x202 ($vx+10) ($vy+10)
        [SnapZyTests.Native]::PostMessage($preview,0x100,[IntPtr](27),[IntPtr]::Zero) | Out-Null
        if([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -ne '160 x 80 px') {throw 'Cancelled crop changed dimensions.'}
        Mouse $preview 0x201 ($vx+100) ($vy+70)
        Mouse $preview 0x202 ($vx+10) ($vy+10)
        [SnapZyTests.Native]::PostMessage($preview,0x100,[IntPtr](13),[IntPtr]::Zero) | Out-Null
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -eq '90 x 60 px'} 'Crop did not change dimensions.' | Out-Null
        $cropped=Save-Png $preview 'cropped.png'
        $image=[Drawing.Bitmap]::FromFile($cropped)
        try {if($image.Width -ne 90 -or $image.Height -ne 60) {throw 'Crop exported wrong dimensions.'}}finally{$image.Dispose()}
        Click $preview 607
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -eq '160 x 80 px'} 'Undo crop did not restore dimensions.' | Out-Null
        Click $preview 608
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -eq '90 x 60 px'} 'Redo crop did not restore dimensions.' | Out-Null
        Save-Png $preview 'cropped-redo.png' | Out-Null
        Click $preview 201
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -eq 'Copied'} 'Composited clipboard copy failed.' | Out-Null
        $clipboard=[Windows.Forms.Clipboard]::GetImage()
        if($null -eq $clipboard) {
            $formats=[Windows.Forms.Clipboard]::GetDataObject().GetFormats() -join ', '
            if(-not [SnapZyTests.Native]::OpenClipboard([IntPtr]::Zero)) {throw 'Cannot inspect clipboard.'}
            try {
                $dib=[SnapZyTests.Native]::GetClipboardData(8)
                $handle=[SnapZyTests.Native]::GetClipboardData(2)
                if($handle -ne [IntPtr]::Zero) {
                    $nativeImage=[Drawing.Image]::FromHbitmap($handle)
                    $nativeSize="$($nativeImage.Width)x$($nativeImage.Height)"
                    $nativeImage.Dispose()
                }else{$nativeSize='unavailable'}
                throw "Clipboard contains no managed image. Available formats: $formats; native DIB=$dib; native bitmap=$handle size=$nativeSize"
            }finally{[SnapZyTests.Native]::CloseClipboard() | Out-Null}
        }
        $bitmap=[Drawing.Bitmap]::new($clipboard)
        try {
            if($bitmap.Width -ne 90 -or $bitmap.Height -ne 60) {throw 'Clipboard dimensions do not match crop.'}
            if($bitmap.GetPixel(30,19).ToArgb() -ne [Drawing.Color]::FromArgb(0,160,80).ToArgb()) {throw 'Clipboard omitted annotations.'}
        }finally{$bitmap.Dispose();$clipboard.Dispose()}
        # Check compact layout and that controls stay within the client on all monitors.
        $dpis=@()
        foreach($monitor in [Windows.Forms.Screen]::AllScreens) {
            $area=$monitor.WorkingArea
            [SnapZyTests.Native]::SetWindowPos($preview,[IntPtr](-1),$area.Left+20,$area.Top+20,700,450,0) | Out-Null
            [SnapZyTests.Native]::UpdateWindow($preview) | Out-Null
            $r=[SnapZyTests.Native]::Client($preview)
            $dpis+=[SnapZyTests.Native]::GetDpiForWindow($preview)
            Click $preview 624
            $boxes=@()
            foreach($id in @(601,602,603,604,605,606,607,608,609,610,611,612,613,614,615,617,618,619,620,621,622,623,624,625,626,627,628,629,630,631,632,633,634,635,636,637,638,639,201,202)) {
                if(-not [SnapZyTests.Native]::IsWindowVisible([SnapZyTests.Native]::GetDlgItem($preview,$id))){continue}
                $box=New-Object SnapZyTests.Native+Rect
                [SnapZyTests.Native]::GetWindowRect([SnapZyTests.Native]::GetDlgItem($preview,$id),[ref]$box) | Out-Null
                if($box.Left -lt $r.Left -or $box.Top -lt $r.Top -or $box.Right -gt $r.Right -or $box.Bottom -gt $r.Bottom) {throw "Control $id exceeds the compact editor bounds."}
                foreach($previous in $boxes) {
                    if($box.Left -lt $previous.Rect.Right -and $box.Right -gt $previous.Rect.Left -and $box.Top -lt $previous.Rect.Bottom -and $box.Bottom -gt $previous.Rect.Top){throw "Controls $id and $($previous.Id) overlap in the compact editor."}
                }
                $boxes+=@{Id=$id;Rect=$box}
            }
        }
        $measurements.tested_monitor_dpis=$dpis
        Snapshot-Editor $preview 'editor-compact.png'
        $measurements.editor='PASS'
        if($OcrChecks) {
            Click $preview 632
            $ocr=Wait-For {Window-Of 'SnapZyRustPreview.Ocr'} 'OCR result window did not open.'
            foreach($id in @(901,902,903,904,905,906,907)) {
                if([SnapZyTests.Native]::GetDlgItem($ocr,$id) -eq [IntPtr]::Zero) {throw "OCR control $id is missing."}
            }
            if([SnapZyTests.Native]::SendMessage([SnapZyTests.Native]::GetDlgItem($ocr,901),0x0147,[IntPtr]::Zero,[IntPtr]::Zero).ToInt64() -ne 0) {throw 'Thai + English is not the default OCR language.'}
            if([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($ocr,905)) -ne '') {throw 'Opening OCR unexpectedly started recognition or populated output.'}
            Click $ocr 904
            Wait-For {-not (Window-Of 'SnapZyRustPreview.Ocr')} 'OCR result window did not close.' | Out-Null
            $measurements.ocr_window='PASS'
        }
    }
    Close-Preview $preview

    $fixture.Invalidate();$fixture.Update();[Windows.Forms.Application]::DoEvents()
    $preflightPixel=[Drawing.Bitmap]::new(1,1)
    $preflightGraphics=[Drawing.Graphics]::FromImage($preflightPixel)
    try {
        Wait-For {
            [SnapZyTests.Native]::DwmFlush() | Out-Null
            $sourceBounds=[SnapZyTests.Native]::Client($fixture.Handle)
            $preflightGraphics.CopyFromScreen($sourceBounds.Left+100,$sourceBounds.Top+80,0,0,$preflightPixel.Size)
            $preflightPixel.GetPixel(0,0).ToArgb() -eq $fixture.BackColor.ToArgb()
        } 'Window fixture is obscured before capture; test setup is not ready.' | Out-Null
    }finally{$preflightGraphics.Dispose();$preflightPixel.Dispose()}
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
    if($AnnotationChecks) {
        $r=[SnapZyTests.Native]::Client($preview)
        $sw=[SnapZyTests.Native]::GetSystemMetrics(78);$sh=[SnapZyTests.Native]::GetSystemMetrics(79)
        $ratio=[Math]::Min(1.0,[Math]::Min(($r.Right-$r.Left-32)/$sw,($r.Bottom-$r.Top-175)/$sh))
        $vx=[Math]::Round(($r.Right-$r.Left-$sw*$ratio)/2)
        $vy=[Math]::Round(145+($r.Bottom-$r.Top-175-$sh*$ratio)/2)
        Click $preview 619
        Mouse $preview 0x201 ([int]($vx+10*$ratio)) ([int]($vy+10*$ratio))
        Mouse $preview 0x200 ([int]($vx+[Math]::Min(1600,$sw-10)*$ratio)) ([int]($vy+[Math]::Min(900,$sh-10)*$ratio))
        $timer=[Diagnostics.Stopwatch]::StartNew()
        Mouse $preview 0x202 ([int]($vx+[Math]::Min(1600,$sw-10)*$ratio)) ([int]($vy+[Math]::Min(900,$sh-10)*$ratio))
        Wait-For {-not [SnapZyTests.Native]::IsWindowEnabled([SnapZyTests.Native]::GetDlgItem($preview,201))} 'Copy was not disabled during large composition.' | Out-Null
        if([SnapZyTests.Native]::IsWindowEnabled([SnapZyTests.Native]::GetDlgItem($preview,202))){throw 'Save was still enabled during composition.'}
        $ping=[Diagnostics.Stopwatch]::StartNew();$reply=[IntPtr]::Zero
        if([SnapZyTests.Native]::SendMessageTimeout($preview,0,[IntPtr]::Zero,[IntPtr]::Zero,2,1000,[ref]$reply) -eq [IntPtr]::Zero){throw 'Large blur blocked the editor message loop.'}
        $measurements.large_blur_ui_ping_ms=[Math]::Round($ping.Elapsed.TotalMilliseconds,3)
        Wait-For {[SnapZyTests.Native]::IsWindowEnabled([SnapZyTests.Native]::GetDlgItem($preview,201))} 'Large blur did not finish.' 30000 | Out-Null
        $measurements.large_blur_ready_ms=[Math]::Round($timer.Elapsed.TotalMilliseconds,3)
        $measurements.large_source_dimensions="$sw x $sh"
    }
    Close-Preview $preview

    if ($ScrollChecks -or $AutoScrollChecks -or $DetectStallChecks) {
        function Body-Pixel([int]$Row,[int]$X) {
            $cell=[int][Math]::Floor($Row/37)+[int][Math]::Floor($Row/83)
            if($Row%37 -eq 0 -or $X%61 -eq 0){return [Drawing.Color]::FromArgb(30,30,30)}
            if(($X*19+$cell*53)%97 -lt 16 -and $Row%37 -gt 8 -and $Row%37 -lt 27){return [Drawing.Color]::FromArgb(50,$cell%137,$Row%127)}
            return [Drawing.Color]::FromArgb(250,250,250)
        }
        function Set-ScrollFixture([int]$Offset,[switch]$Fast) {
            if($Fast -or $AutoScrollChecks) {
                $bitmap=[SnapZyTests.ScrollFixture]::Image($Offset,$fixture.BackColor)
                $old=$fixture.BackgroundImage;$fixture.BackgroundImage=$bitmap
                if($old){$old.Dispose()}
                $fixture.Invalidate();$fixture.Update()
                return
            }
            $bitmap=[Drawing.Bitmap]::new(240,400)
            $g=[Drawing.Graphics]::FromImage($bitmap)
            try {$g.Clear($fixture.BackColor)}finally{$g.Dispose()}
            for($sy=0;$sy -lt 400;$sy++){for($sx=0;$sx -lt 240;$sx++){
                if($sy -lt 55){$color=[Drawing.Color]::FromArgb(45,130,15)}
                elseif($sy -ge 358){
                    if($sy -eq 358 -or ($sy -ge 363 -and $sx -lt 120)){$color=[Drawing.Color]::FromArgb(190,190,190)}
                    else{$color=[Drawing.Color]::FromArgb(245,245,245)}
                }else{$color=Body-Pixel ($sy-55+$Offset) $sx}
                $bitmap.SetPixel($sx,$sy,$color)
            }}
            $old=$fixture.BackgroundImage
            $fixture.BackgroundImage=$bitmap
            if($old){$old.Dispose()}
            $fixture.Invalidate();$fixture.Update();[Windows.Forms.Application]::DoEvents()
            [SnapZyTests.Native]::DwmFlush() | Out-Null
        }
        function Begin-ScrollFixture([bool]$Automatic=$false,[switch]$EscapeChecks,[switch]$EscapeDuringDetection,[switch]$ExpectTimeout) {
            $choice=if($Automatic){110}else{111}
            if([SnapZyTests.Native]::GetDlgItem($main,$choice) -ne [IntPtr]::Zero) {
                Click $main $choice
                Wait-For {[SnapZyTests.Native]::SendMessage([SnapZyTests.Native]::GetDlgItem($main,$choice),0xf0,[IntPtr]::Zero,[IntPtr]::Zero).ToInt64() -eq 1} 'Scroll mode choice did not persist.' | Out-Null
                $stored=Get-Content -LiteralPath (Join-Path $runDirectory 'settings.json') -Raw | ConvertFrom-Json
                if($stored.scroll_automatic -ne $Automatic){throw 'Scrolling mode was not saved in Rust settings.'}
            }
            [SnapZyTests.Native]::SetWindowPos($fixture.Handle,[IntPtr](-1),0,0,0,0,3) | Out-Null
            $script:fixtureScrollOffset=0
            $fixture.Show();$fixture.Update();[Windows.Forms.Application]::DoEvents()
            [SnapZyTests.Native]::ConfigureVerticalScroll($fixture.Handle)
            if($ExpectTimeout){[SnapZyTests.Native]::ClearVerticalScroll($fixture.Handle)}
            [SnapZyTests.Native]::ShowWindow($main,9) | Out-Null
            [SnapZyTests.Native]::SetWindowPos($main,[IntPtr](-1),0,0,0,0,3) | Out-Null
            [SnapZyTests.Native]::UpdateWindow($main) | Out-Null
            [SnapZyTests.Native]::DwmFlush() | Out-Null
            $button=New-Object SnapZyTests.Native+Rect
            [SnapZyTests.Native]::GetWindowRect([SnapZyTests.Native]::GetDlgItem($main,109),[ref]$button) | Out-Null
            $clickPoint=[SnapZyTests.Native+Point]::new();$clickPoint.X=[int](($button.Left+$button.Right)/2);$clickPoint.Y=[int](($button.Top+$button.Bottom)/2)
            if($MessageOnly){Click $main 109}
            else{[SnapZyTests.Native]::InputAt($main,$process.Id,$clickPoint,$true,0)}
            $o=Wait-For {Window-Of 'SnapZyRustPreview.Selection'} 'Scrolling selection did not open.'
            Wait-For {[SnapZyTests.Native]::IsWindowVisible($o)} 'Scrolling selection was not visible.' | Out-Null
            $bounds=[SnapZyTests.Native]::Client($fixture.Handle)
            $x=$bounds.Left+120-[SnapZyTests.Native]::GetSystemMetrics(76)
            $y=$bounds.Top+200-[SnapZyTests.Native]::GetSystemMetrics(77)
            $startButton=[SnapZyTests.Native]::GetDlgItem($o,804)
            [SnapZyTests.Native]::UpdateWindow($startButton) | Out-Null
            Snapshot-Editor $o 'detect-before-move.png' -KeepPosition
            $coordinates=([int64]($y-band 0xffff)-shl 16)-bor ($x-band 0xffff)
            [SnapZyTests.Native]::SendMessage($o,0x200,[IntPtr]::Zero,[IntPtr]$coordinates) | Out-Null
            if($EscapeDuringDetection) {
                $status=[SnapZyTests.Native]::GetDlgItem($o,113)
                Wait-For {[SnapZyTests.Native]::Text($status) -eq 'Checking the selected area...'} 'Hover did not enter its progress state.' | Out-Null
                if(-not [SnapZyTests.Native]::IsWindowVisible($o)){throw 'Pending hover hid the selection overlay.'}
                Mouse $o 0x201 $x $y
                Mouse $o 0x202 $x $y
                if(Window-Of 'SnapZyRustPreview.Scroll'){throw 'A click before a ready frame started capture.'}
                if($MessageOnly){[SnapZyTests.Native]::PostMessage($o,0x100,[IntPtr](27),[IntPtr]::Zero) | Out-Null}
                else{[SnapZyTests.Native]::PressEscape()}
                Wait-For {-not (Window-Of 'SnapZyRustPreview.Selection')} 'Escape did not cancel pending hover detection.' | Out-Null
                return $null
            }
            if(-not $ExpectTimeout -and -not $EscapeDuringDetection){
                Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($o,113)).StartsWith('Found:')} 'Hover did not detect a frame before any click.' 7000 | Out-Null
                if(-not [SnapZyTests.Native]::IsWindowVisible($o)){throw 'Hover detection hid the overlay.'}
                if([SnapZyTests.Native]::GetForegroundWindow() -eq $o){throw 'Hover detection stole the target focus.'}
                if($script:fixtureScrollOffset -ne 0){throw 'Hover detection scrolled before confirmation.'}
            }
            Snapshot-Editor $o 'detect-before-move.png' -KeepPosition
            [SnapZyTests.Native]::SendMessage($o,0x200,[IntPtr]::Zero,[IntPtr]$coordinates) | Out-Null
            Snapshot-Editor $o 'detect-after-move.png' -KeepPosition
            $buttonBounds=[SnapZyTests.Native+Rect]::new()
            [SnapZyTests.Native]::GetWindowRect($startButton,[ref]$buttonBounds) | Out-Null
            $before=[Drawing.Bitmap]::FromFile((Join-Path $runDirectory 'detect-before-move.png'))
            $after=[Drawing.Bitmap]::FromFile((Join-Path $runDirectory 'detect-after-move.png'))
            try {
                for($py=$buttonBounds.Top-[SnapZyTests.Native]::GetSystemMetrics(77);$py -lt $buttonBounds.Bottom-[SnapZyTests.Native]::GetSystemMetrics(77);$py++) {
                    for($px=$buttonBounds.Left-[SnapZyTests.Native]::GetSystemMetrics(76);$px -lt $buttonBounds.Right-[SnapZyTests.Native]::GetSystemMetrics(76);$px++) {
                        if($before.GetPixel($px,$py).ToArgb() -ne $after.GetPixel($px,$py).ToArgb()){throw 'Moving over the detection overlay erased its Start control.'}
                    }
                }
            } finally {$before.Dispose();$after.Dispose()}
            $status=[SnapZyTests.Native]::GetDlgItem($o,113)
            if($ExpectTimeout) {
                Wait-For {[SnapZyTests.Native]::Text($status).StartsWith('Window frame:')} 'Stalled detector did not produce a labeled .NET-style window fallback.' 7000 | Out-Null
                if(-not [SnapZyTests.Native]::IsWindowVisible($o) -or -not [SnapZyTests.Native]::IsWindowEnabled($startButton)){throw 'Timeout left an invisible overlay or unusable fallback.'}
                Mouse $o 0x200 ($x+2) $y
                Wait-For {
                    $lines=@(Get-ChildItem (Join-Path $runDirectory 'Logs') -Filter 'performance-*.log' | Get-Content)
                    @($lines | Where-Object {$_ -match 'scroll.detect.timeout'}).Count -ge 2
                } 'Second request remained stuck behind the timed-out provider.' 7000 | Out-Null
                if(-not [SnapZyTests.Native]::Text($status).StartsWith('Window frame:')){throw 'Timeout discarded the visible fallback frame.'}
                [SnapZyTests.Native]::PostMessage($o,0x100,[IntPtr](27),[IntPtr]::Zero) | Out-Null
                Wait-For {-not (Window-Of 'SnapZyRustPreview.Selection')} 'Could not cancel after two provider timeouts.' | Out-Null
                $measurements.detection_timeout_recovery='PASS: two blocked providers reaped, labeled window fallback after timeout, hover retry accepted, overlay stayed visible, cancellation succeeded'
                return $null
            }
            Wait-For {[SnapZyTests.Native]::Text($status).StartsWith('Found:')} 'Hover did not identify the native vertical scroll pane.' 7000 | Out-Null
            if($EscapeChecks) {
                if($MessageOnly){[SnapZyTests.Native]::PostMessage($o,0x100,[IntPtr](27),[IntPtr]::Zero) | Out-Null}
                else{[SnapZyTests.Native]::PressEscape()}
                Wait-For {-not (Window-Of 'SnapZyRustPreview.Selection')} 'Escape did not cancel the detected scrolling pane.' | Out-Null
                if(Window-Of 'SnapZyRustPreview.Scroll'){throw 'Escape started scrolling instead of cancelling detection.'}
                return $null
            }
            if($script:fixtureScrollOffset -ne 0){throw "Detection changed the source scroll position before confirmation: offset=$script:fixtureScrollOffset"}
            [SnapZyTests.Native]::SendMessage($o,0x201,[IntPtr](1),[IntPtr]$coordinates) | Out-Null
            if(-not [SnapZyTests.Native]::IsWindowVisible($o) -or (Window-Of 'SnapZyRustPreview.Scroll')){throw 'Capture began before the confirmation click was released.'}
            Mouse $o 0x202 $x $y
            $controller=Wait-For {Window-Of 'SnapZyRustPreview.Scroll'} "Scrolling controller did not open. Overlay=$([SnapZyTests.Native]::Text($status)); StartEnabled=$([SnapZyTests.Native]::IsWindowEnabled([SnapZyTests.Native]::GetDlgItem($o,804))); Windows=$([SnapZyTests.Native]::ProcessWindows($process.Id))"
            Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($controller,401)).EndsWith('400 px')} 'Scrolling first stable frame not accepted.' 15000 | Out-Null
            return $controller
        }
        $fixture.ClientSize=[Drawing.Size]::new(240,400)
        $scrollViewportWidth=240-[SnapZyTests.Native]::GetSystemMetrics(2)
        $script:fixtureScrollOffset=0
        if($DetectStallChecks) {
            Set-ScrollFixture 0
            Begin-ScrollFixture -ExpectTimeout | Out-Null
        }
        if ($ScrollChecks) {
        $wheelHandler=[Windows.Forms.MouseEventHandler]{param($sender,$event)
            if($event.Delta -lt 0){$script:fixtureScrollOffset += [int](-$event.Delta/120)}
        }
        $fixture.Add_MouseWheel($wheelHandler)
        if($InputProbe) {
            [SnapZyTests.Native]::SetWindowPos($fixture.Handle,[IntPtr](-1),0,0,0,0,3) | Out-Null
            $preflight=[SnapZyTests.Native]::Client($fixture.Handle)
            $point=[SnapZyTests.Native+Point]::new();$point.X=$preflight.Left+100;$point.Y=$preflight.Top+180
            [SnapZyTests.Native]::InputAt($fixture.Handle,$PID,$point,$true,0)
            Wait-For {[SnapZyTests.Native]::GetForegroundWindow() -eq $fixture.Handle} 'Test setup cannot activate its source fixture.' | Out-Null
            [SnapZyTests.Native]::InputAt($fixture.Handle,$PID,$point,$false,-120)
            Wait-For {$script:fixtureScrollOffset -eq 1} 'Test setup cannot receive wheel input before capture.' | Out-Null
            $script:fixtureScrollOffset=0
        }
        Set-ScrollFixture 0
        Begin-ScrollFixture -EscapeDuringDetection | Out-Null
        Set-ScrollFixture 0
        Begin-ScrollFixture -EscapeChecks | Out-Null
        $controller=Begin-ScrollFixture
        if(Window-Of 'SnapZyRustPreview.Selection'){throw 'Selection overlay still exists during scrolling.'}
        if([SnapZyTests.Native]::MouseCapture($main) -ne [IntPtr]::Zero){throw 'SnapZy retained mouse capture after scrolling selection.'}
        $activation=[SnapZyTests.Native]::SendMessage($controller,0x0021,$controller,[IntPtr]((0x0201-shl 16)-bor 1))
        if($activation.ToInt64() -ne 3){throw 'Scrolling controller permits a mouse click to steal source focus.'}
        Wait-For {[SnapZyTests.Native]::GetForegroundWindow() -eq $fixture.Handle} "Scrolling did not return foreground to target. Current=$([SnapZyTests.Native]::GetForegroundWindow()) class=$([SnapZyTests.Native]::Class([SnapZyTests.Native]::GetForegroundWindow())) target=$($fixture.Handle)" | Out-Null
        $controlBounds=[SnapZyTests.Native]::Client($controller)
        $statusPoint=[SnapZyTests.Native+Point]::new();$statusPoint.X=$controlBounds.Left+20;$statusPoint.Y=$controlBounds.Top+20
        if(-not $MessageOnly){[SnapZyTests.Native]::InputAt($controller,$process.Id,$statusPoint,$true,0)}
        [Windows.Forms.Application]::DoEvents()
        if([SnapZyTests.Native]::GetForegroundWindow() -ne $fixture.Handle){throw 'Clicking the scrolling controller stole target focus.'}
        $originalLocation=$fixture.Location
        $fixture.Location=[Drawing.Point]::new($originalLocation.X+20,$originalLocation.Y)
        [Windows.Forms.Application]::DoEvents()
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($controller,401)).StartsWith('Paused: target')} 'Moving target did not pause scrolling.' | Out-Null
        $fixture.Location=$originalLocation
        [Windows.Forms.Application]::DoEvents()
        foreach($offset in @(94,181,253)) {
            $bounds=[SnapZyTests.Native]::Client($fixture.Handle)
            $point=[SnapZyTests.Native+Point]::new();$point.X=$bounds.Left+100;$point.Y=$bounds.Top+180
            $wheelTarget=[SnapZyTests.Native]::WindowFromPoint($point)
            if([SnapZyTests.Native]::GetAncestor($wheelTarget,2) -ne $fixture.Handle){throw 'SnapZy or another window covers the scrolling source.'}
            $delta=-120*($offset-$script:fixtureScrollOffset)
            if($InputProbe){[SnapZyTests.Native]::InputAt($fixture.Handle,$PID,$point,$false,$delta)}
            else{
                $coordinates=([int64]($point.Y-band 0xffff)-shl 16)-bor ($point.X-band 0xffff)
                [SnapZyTests.Native]::SendMessage($wheelTarget,0x020a,[IntPtr]([int64]$delta-shl 16),[IntPtr]$coordinates) | Out-Null
            }
            Wait-For {$script:fixtureScrollOffset -eq $offset} 'Source window did not receive downward wheel input.' | Out-Null
            Set-ScrollFixture $script:fixtureScrollOffset
            $height=400+$offset
            try {
                Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($controller,401)).EndsWith("$height px")} "Scroll offset $offset not accepted." 15000 | Out-Null
            }catch{
                $state=[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($controller,401))
                Snapshot-Editor $controller 'scroll-status-failed.png' -KeepPosition
                Snapshot-Editor $fixture.Handle 'scroll-source-failed.png' -KeepPosition
                throw "Scroll offset $offset not accepted. Status=$state Evidence=$runDirectory"
            }
        }
        Snapshot-Editor $controller 'scroll-controller.png' -KeepPosition
        $escapeAvailable=[SnapZyTests.Native]::RegisterHotKey([IntPtr]::Zero,9882,0x4000,27)
        if($escapeAvailable){
            [SnapZyTests.Native]::UnregisterHotKey([IntPtr]::Zero,9882) | Out-Null
            throw 'Active scrolling did not reserve Escape while the source owns focus.'
        }
        if($EscapeInputChecks){[SnapZyTests.Native]::EscapeAt($fixture.Handle,$PID)}
        else{[SnapZyTests.Native]::PostMessage($controller,0x312,[IntPtr](0x5343),[IntPtr]::Zero) | Out-Null}
        $preview=Wait-For {Window-Of 'SnapZyRustPreview.Capture'} 'Global Escape did not preserve scrolling result.' 15000
        $escapeReleased=[SnapZyTests.Native]::RegisterHotKey([IntPtr]::Zero,9882,0x4000,27)
        if(-not $escapeReleased){throw 'Scrolling did not release Escape after stopping.'}
        [SnapZyTests.Native]::UnregisterHotKey([IntPtr]::Zero,9882) | Out-Null
        $png=Save-Png $preview 'scroll-frozen.png'
        $image=[Drawing.Bitmap]::FromFile($png)
        try {
            if($image.Width -ne $scrollViewportWidth -or $image.Height -ne 653){throw "Scrolling result dimensions changed: $($image.Width)x$($image.Height), expected $scrollViewportWidth x 653."}
            for($sy=0;$sy -lt 646;$sy++){for($sx=0;$sx -lt $scrollViewportWidth;$sx++){
                if($sy -lt 55){$color=[Drawing.Color]::FromArgb(45,130,15)}
                elseif($sy -ge 611){
                    if($sy -eq 611 -or ($sy -ge 616 -and $sx -lt 120)){$color=[Drawing.Color]::FromArgb(190,190,190)}
                    else{$color=[Drawing.Color]::FromArgb(245,245,245)}
                }else{$color=Body-Pixel ($sy-55) $sx}
                if($image.GetPixel($sx,$sy).ToArgb() -ne $color.ToArgb()){throw "Scrolling seam or pixel mismatch at $sx,$sy"}
            }}
        }finally{$image.Dispose()}
        Click $preview 201
        Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -eq 'Copied'} 'Scrolling Copy failed.' | Out-Null
        $copied=[Windows.Forms.Clipboard]::GetImage()
        if(-not $copied){throw 'Scrolling image missing from actual clipboard.'}
        $a=[Drawing.Bitmap]::FromFile($png);$b=[Drawing.Bitmap]::new($copied)
        try {
            if($a.Width -ne $b.Width -or $a.Height -ne $b.Height){throw 'Scrolling clipboard dimensions differ.'}
            for($sy=0;$sy -lt 653;$sy++){for($sx=0;$sx -lt $scrollViewportWidth;$sx++){
                if($a.GetPixel($sx,$sy).ToArgb() -ne $b.GetPixel($sx,$sy).ToArgb()){throw "Scrolling clipboard pixels differ at $sx,$sy"}
            }}
        }finally{$a.Dispose();$b.Dispose();$copied.Dispose()}
        Close-Preview $preview
        Set-ScrollFixture 0
        $controller=Begin-ScrollFixture
        Click $controller 802
        Wait-For {-not (Window-Of 'SnapZyRustPreview.Scroll')} 'Scrolling cancel did not close.' | Out-Null
        Start-Sleep -Milliseconds 700
        if(Window-Of 'SnapZyRustPreview.Capture'){throw 'Cancelled scrolling session opened a late preview.'}
        $fixture.BackgroundImage.Dispose();$fixture.BackgroundImage=$null
        $fixture.Remove_MouseWheel($wheelHandler)
        $measurements.scrolling='PASS'
        $measurements.scroll_manual_wheel_handoff='PASS'
        $measurements.scrolling_escape='PASS: active registration, Stop preserves accepted PNG/clipboard pixels, registration released afterwards'
        $measurements.scrolling_escape_delivery=if($EscapeInputChecks){'OS-injected Escape down/up while the test-owned source retains focus'}else{'Posted hotkey message; physical key delivery not tested'}
        $measurements.scroll_os_wheel_probe=if($InputProbe){'PASS'}else{'NOT RUN; separate -InputProbe preflight is required'}
        }
        if ($AutoScrollChecks) {
            $script:fixtureScrollOffset=0;$script:autoWheelCount=0
            $script:autoRenderPending=$false
            $renderTimer=[Windows.Forms.Timer]::new();$renderTimer.Interval=900
            $renderTimer.Add_Tick({
                $renderTimer.Stop()
                Set-ScrollFixture $script:fixtureScrollOffset -Fast
                $script:autoRenderPending=$false
            })
            $autoHandler=[Windows.Forms.MouseEventHandler]{param($sender,$event)
                if($event.Delta -lt 0) {
                    if($script:autoRenderPending){throw 'Product scrolled again before delayed fixture render completed.'}
                    $script:autoWheelCount++
                    $next=[Math]::Min(253,$script:fixtureScrollOffset+60)
                    if($next -ne $script:fixtureScrollOffset) {
                        $script:fixtureScrollOffset=$next
                        $script:autoRenderPending=$true
                        $renderTimer.Start()
                    }
                }
            }
            $fixture.Add_MouseWheel($autoHandler)
            foreach($stopKind in @('Stop','Escape')) {
                $script:fixtureScrollOffset=0;$script:autoWheelCount=0;$script:autoRenderPending=$false
                Set-ScrollFixture 0 -Fast
                $controller=Begin-ScrollFixture $true
                Wait-For {$script:autoWheelCount -eq 1 -and $script:autoRenderPending} 'Tail fixture did not have one pending delayed scroll.' | Out-Null
                $screen=[Drawing.Bitmap]::new($scrollViewportWidth,400)
                $g=[Drawing.Graphics]::FromImage($screen)
                try {
                    $bounds=[SnapZyTests.Native]::Client($fixture.Handle)
                    $g.CopyFromScreen($bounds.Left,$bounds.Top,0,0,$screen.Size)
                }finally{$g.Dispose()}
                $stopWatch=[Diagnostics.Stopwatch]::StartNew()
                if($stopKind -eq 'Stop'){Click $controller 801}
                elseif($EscapeInputChecks){[SnapZyTests.Native]::EscapeAt($fixture.Handle,$PID)}
                else{[SnapZyTests.Native]::PostMessage($controller,0x312,[IntPtr](0x5343),[IntPtr]::Zero) | Out-Null}
                $preview=Wait-For {Window-Of 'SnapZyRustPreview.Capture'} "$stopKind did not finish its pending final step." 15000
                $measurements["tail_${stopKind}_preview_ms"]=[Math]::Round($stopWatch.Elapsed.TotalMilliseconds,3)
                $label=[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401))
                if($label -ne "$scrollViewportWidth x 460 px"){throw "$stopKind lost its pending final scroll: expected $scrollViewportWidth x 460 px, got '$label'; wheels=$script:autoWheelCount offset=$script:fixtureScrollOffset pending=$script:autoRenderPending"}
                if($script:autoWheelCount -ne 1 -or $script:autoRenderPending){throw "$stopKind sent extra input or completed before delayed source rendering."}
                $tail=Save-Png $preview "auto-tail-$stopKind.png"
                $image=[Drawing.Bitmap]::FromFile($tail)
                try {
                    if($image.Width -ne $scrollViewportWidth -or $image.Height -ne 460){throw "$stopKind tail PNG dimensions differ from its preview."}
                    for($sy=0;$sy -lt 460;$sy++){for($sx=0;$sx -lt $scrollViewportWidth;$sx++){
                        if($sy -lt 55){$color=[Drawing.Color]::FromArgb(45,130,15)}
                        elseif($sy -ge 418){$color=$screen.GetPixel($sx,$sy-60)}
                        else{$color=Body-Pixel ($sy-55) $sx}
                        if($image.GetPixel($sx,$sy).ToArgb() -ne $color.ToArgb()){throw "$stopKind final-step pixel mismatch at $sx,$sy"}
                    }}
                }finally{$screen.Dispose();$image.Dispose()}
                Close-Preview $preview
                $measurements["tail_$stopKind"]='PASS: stopped during delayed render, exact full body/footer, one wheel only'
            }
            $script:fixtureScrollOffset=0;$script:autoWheelCount=0;$script:autoRenderPending=$false
            Set-ScrollFixture 0 -Fast
            $controller=Begin-ScrollFixture $true
            Wait-For {$script:autoWheelCount -eq 1 -and $script:autoRenderPending} 'Cancel-tail fixture has no pending scroll.' | Out-Null
            Click $controller 801
            Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($controller,401)).StartsWith('Finishing the last frame')} 'Stop did not expose finishing status.' | Out-Null
            Click $controller 802
            Wait-For {-not (Window-Of 'SnapZyRustPreview.Scroll')} 'Cancel did not close finishing controller.' | Out-Null
            Start-Sleep -Milliseconds 1300;[Windows.Forms.Application]::DoEvents()
            if($script:autoWheelCount -ne 1 -or (Window-Of 'SnapZyRustPreview.Capture')){throw 'Cancel during final sampling sent another wheel or opened a late preview.'}
            $measurements.tail_cancel='PASS: Cancel during finishing, no extra wheel or late preview'

            $renderTimer.Interval=3500
            $script:fixtureScrollOffset=0;$script:autoWheelCount=0;$script:autoRenderPending=$false
            Set-ScrollFixture 0 -Fast
            $controller=Begin-ScrollFixture $true
            Wait-For {$script:autoWheelCount -eq 1 -and $script:autoRenderPending} 'Timeout-tail fixture has no pending scroll.' | Out-Null
            $stopWatch=[Diagnostics.Stopwatch]::StartNew()
            Click $controller 801
            $dialog=Wait-For {
                if([SnapZyTests.Native]::IsWindowVisible($controller)){Click $controller 801}
                Window-Of '#32770'
            } 'Repeated Stop prevented the final-frame timeout.' 5000
            $measurements.tail_timeout_ms=[Math]::Round($stopWatch.Elapsed.TotalMilliseconds,3)
            if($stopWatch.Elapsed.TotalMilliseconds -ge 3300){throw 'Repeated Stop restarted the bounded finishing deadline.'}
            $messages=Get-Content -LiteralPath (Join-Path $runDirectory 'Logs/capture-diagnostics.log')
            if(-not ($messages | Where-Object {$_ -like 'notification error=false text=Could not confirm the final frame in time.*'})){throw 'Tail timeout did not explain its partial result.'}
            Dismiss-Message $dialog 1
            $preview=Wait-For {Window-Of 'SnapZyRustPreview.Capture'} 'Timeout lost its valid partial preview.'
            if([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -ne "$scrollViewportWidth x 400 px"){throw 'Tail timeout fabricated or duplicated content.'}
            $renderTimer.Stop();$script:autoRenderPending=$false;$renderTimer.Interval=900
            Close-Preview $preview
            if($script:autoWheelCount -ne 1){throw 'Timeout sent another wheel.'}
            $measurements.tail_timeout='PASS: unchanged old frames wait, bounded deadline despite repeated Stop, explicit partial notice'

            $script:fixtureScrollOffset=0;$script:autoWheelCount=0;$script:autoRenderPending=$false
            Set-ScrollFixture 0 -Fast
            $controller=Begin-ScrollFixture $true
            Wait-For {$script:autoWheelCount -eq 1 -and $script:autoRenderPending} 'Covered-tail fixture has no pending scroll.' | Out-Null
            Click $controller 801
            Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($controller,401)).StartsWith('Finishing the last frame')} 'Covered-tail Stop did not start finishing.' | Out-Null
            $cover=[Windows.Forms.Form]::new()
            try {
                $cover.Text='';$cover.StartPosition='Manual';$cover.FormBorderStyle='None'
                $bounds=[SnapZyTests.Native]::Client($fixture.Handle)
                $cover.Bounds=[Drawing.Rectangle]::new($bounds.Left+5,$bounds.Top+80,50,50)
                [SnapZyTests.Native]::SetWindowPos($cover.Handle,[IntPtr](-1),0,0,0,0,0x53) | Out-Null
                [SnapZyTests.Native]::UpdateWindow($cover.Handle) | Out-Null
                $dialog=Wait-For {Window-Of '#32770'} 'Covered final frame did not explain its partial result.'
                $messages=Get-Content -LiteralPath (Join-Path $runDirectory 'Logs/capture-diagnostics.log')
                if(-not ($messages | Where-Object {$_ -like 'notification error=false text=Paused: target inactive, moved or covered*'})){throw 'Covered final frame returned the wrong notice.'}
                Dismiss-Message $dialog 1
                $preview=Wait-For {Window-Of 'SnapZyRustPreview.Capture'} 'Covered target lost its valid partial image.'
                if([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($preview,401)) -ne "$scrollViewportWidth x 400 px"){throw 'Covered final source was appended.'}
                if($script:autoWheelCount -ne 1){throw 'Covered final source received extra input.'}
                Close-Preview $preview
                $measurements.tail_covered='PASS: covered source rejected while finishing, partial notice and no extra wheel'
            }finally{$cover.Close();$cover.Dispose();$renderTimer.Stop();$script:autoRenderPending=$false}
            $script:fixtureScrollOffset=0;$script:autoWheelCount=0;$script:autoRenderPending=$false
            Set-ScrollFixture 0 -Fast
            $controller=Begin-ScrollFixture $true
            Wait-For {$script:autoWheelCount -gt 0} 'Product did not send automatic wheel input.' | Out-Null
            Click $controller 803
            Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($controller,803)) -eq 'Resume'} 'Automatic Pause did not latch.' | Out-Null
            $count=$script:autoWheelCount
            Start-Sleep -Milliseconds 1100;[Windows.Forms.Application]::DoEvents()
            if($script:autoWheelCount -ne $count){throw 'Product sent wheel input while paused.'}
            Snapshot-Editor $controller 'auto-paused.png' -KeepPosition
            Click $controller 803
            Wait-For {$script:autoWheelCount -gt $count} "Resume did not continue automatic scrolling. Status=$([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($controller,401))); wheels=$script:autoWheelCount/$count offset=$script:fixtureScrollOffset pending=$script:autoRenderPending foreground=$([SnapZyTests.Native]::GetForegroundWindow()) target=$($fixture.Handle)" | Out-Null

            # An untitled, test-owned topmost window must pause capture. No
            # injected wheel or forced fixture offset is used in this test.
            $cover=[Windows.Forms.Form]::new()
            try {
                $cover.Text='';$cover.StartPosition='Manual';$cover.FormBorderStyle='None'
                $bounds=[SnapZyTests.Native]::Client($fixture.Handle)
                $cover.Bounds=[Drawing.Rectangle]::new($bounds.Left+5,$bounds.Top+5,50,50)
                # Form.Show can activate despite ShowWithoutActivation on this
                # host. Show the test HWND explicitly without activation.
                [SnapZyTests.Native]::SetWindowPos($cover.Handle,[IntPtr](-1),0,0,0,0,0x53) | Out-Null
                [SnapZyTests.Native]::UpdateWindow($cover.Handle) | Out-Null
                if([SnapZyTests.Native]::GetForegroundWindow() -ne $fixture.Handle){throw "Occlusion-only fixture unexpectedly stole focus. Actual=$([SnapZyTests.Native]::GetForegroundWindow()) source=$($fixture.Handle) cover=$($cover.Handle)"}
                Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($controller,401)).StartsWith('Paused: target')} 'Untitled occlusion did not pause automatic scrolling.' | Out-Null
                $count=$script:autoWheelCount
                Start-Sleep -Milliseconds 900;[Windows.Forms.Application]::DoEvents()
                if($script:autoWheelCount -ne $count){throw 'Automatic input continued after focus loss/occlusion.'}
                [SnapZyTests.Native]::ShowWindow($cover.Handle,0) | Out-Null
                $cover.Close()
                $bounds=[SnapZyTests.Native]::Client($fixture.Handle)
                $point=[SnapZyTests.Native+Point]::new();$point.X=$bounds.Left+100;$point.Y=$bounds.Top+180
                if($MessageOnly){$fixture.Activate()}else{[SnapZyTests.Native]::InputAt($fixture.Handle,$PID,$point,$true,0)}
                Wait-For {[SnapZyTests.Native]::GetForegroundWindow() -eq $fixture.Handle} 'Could not restore test-owned target focus.' | Out-Null
                Start-Sleep -Milliseconds 500;[Windows.Forms.Application]::DoEvents()
                if($script:autoWheelCount -ne $count){throw 'Safety pause resumed without explicit Resume.'}
                Click $controller 803
            }finally{$cover.Dispose()}
            Wait-For {$script:autoWheelCount -gt $count} 'Automatic scrolling did not resume after occlusion.' | Out-Null
            $focus=[Windows.Forms.Form]::new()
            try {
                $focus.Text='Test-owned focus change';$focus.StartPosition='Manual'
                $focus.Bounds=[Drawing.Rectangle]::new($bounds.Right+30,$bounds.Top,150,100)
                $focus.Show();$focus.Activate()
                Wait-For {[SnapZyTests.Native]::GetForegroundWindow() -eq $focus.Handle} 'Focus fixture could not take focus.' | Out-Null
                Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($controller,401)).StartsWith('Paused: target')} 'Focus-only change did not pause automatic scrolling.' | Out-Null
                $count=$script:autoWheelCount
                Start-Sleep -Milliseconds 900;[Windows.Forms.Application]::DoEvents()
                if($script:autoWheelCount -ne $count){throw 'Product sent wheel input to unfocused target.'}
                $focus.Close()
                if($MessageOnly){$fixture.Activate()}else{[SnapZyTests.Native]::InputAt($fixture.Handle,$PID,$point,$true,0)}
                Wait-For {[SnapZyTests.Native]::GetForegroundWindow() -eq $fixture.Handle} 'Could not restore source after focus-only check.' | Out-Null
                Click $controller 803
            }finally{$focus.Dispose()}
            $dialog=Wait-For {Window-Of '#32770'} "No-motion completion did not explain its reason. Status=$([SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($controller,401))); wheels=$script:autoWheelCount offset=$script:fixtureScrollOffset pending=$script:autoRenderPending foreground=$([SnapZyTests.Native]::GetForegroundWindow()) target=$($fixture.Handle); Windows=$([SnapZyTests.Native]::ProcessWindows($process.Id))" 25000
            $messages=Get-Content -LiteralPath (Join-Path $runDirectory 'Logs/capture-diagnostics.log')
            if(-not ($messages | Where-Object {$_ -like 'notification error=false text=No new content confirmed after retries.*'})){throw 'No-motion completion incorrectly claimed end of document.'}
            Dismiss-Message $dialog 1
            $preview=Wait-For {Window-Of 'SnapZyRustPreview.Capture'} 'Automatic capture did not produce an editor result.'
            $png=Save-Png $preview 'auto-frozen.png'
            $image=[Drawing.Bitmap]::FromFile($png)
            try {
                if($image.Width -ne $scrollViewportWidth -or $image.Height -ne 653){throw "Automatic result dimensions wrong: $($image.Width)x$($image.Height); expected=$scrollViewportWidth x 653 offset=$script:fixtureScrollOffset wheels=$script:autoWheelCount"}
                for($sy=0;$sy -lt 646;$sy++){for($sx=0;$sx -lt $scrollViewportWidth;$sx++){
                    if($sy -lt 55){$color=[Drawing.Color]::FromArgb(45,130,15)}
                    elseif($sy -ge 611){$color=if($sy -eq 611 -or ($sy -ge 616 -and $sx -lt 120)){[Drawing.Color]::FromArgb(190,190,190)}else{[Drawing.Color]::FromArgb(245,245,245)}}
                    else{$color=Body-Pixel ($sy-55) $sx}
                    if($image.GetPixel($sx,$sy).ToArgb() -ne $color.ToArgb()){throw "Automatic seam/pixel mismatch at $sx,$sy"}
                }}
            }finally{$image.Dispose()}
            Close-Preview $preview
            $script:fixtureScrollOffset=0;$script:autoWheelCount=0
            Set-ScrollFixture 0 -Fast
            $controller=Begin-ScrollFixture $true
            Click $controller 803
            Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($controller,803)) -eq 'Resume'} 'Cancel test pause did not latch.' | Out-Null
            Click $controller 802
            Wait-For {-not (Window-Of 'SnapZyRustPreview.Scroll')} 'Automatic cancel did not close controller.' | Out-Null
            $count=$script:autoWheelCount
            Start-Sleep -Milliseconds 1500;[Windows.Forms.Application]::DoEvents()
            if($script:autoWheelCount -ne $count -or (Window-Of 'SnapZyRustPreview.Capture')){throw 'Cancelled automatic job sent input or produced a late result.'}
            $script:fixtureScrollOffset=0;$script:autoWheelCount=0
            Set-ScrollFixture 0 -Fast
            $controller=Begin-ScrollFixture $true
            Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($controller,401)).EndsWith('460 px')} 'Automatic Stop fixture did not accept its first scroll.' 15000 | Out-Null
            Click $controller 803
            Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($controller,803)) -eq 'Resume'} 'Automatic Stop fixture did not pause.' | Out-Null
            [SnapZyTests.Native]::PostMessage([SnapZyTests.Native]::GetDlgItem($controller,803),0x100,[IntPtr](27),[IntPtr]::Zero) | Out-Null
            $preview=Wait-For {Window-Of 'SnapZyRustPreview.Capture'} 'Automatic Stop did not preserve partial result.' 15000
            $partial=Save-Png $preview 'auto-stop.png'
            $image=[Drawing.Bitmap]::FromFile($partial)
            try {
                if($image.Width -ne $scrollViewportWidth -or $image.Height -lt 460 -or $image.Height -ge 653){throw 'Automatic Stop did not retain a bounded partial result.'}
                if($image.GetPixel(80,20).ToArgb() -ne [Drawing.Color]::FromArgb(45,130,15).ToArgb()){throw 'Automatic Stop lost original header pixels.'}
            }finally{$image.Dispose()}
            Close-Preview $preview
            if($EscapeInputChecks){
                $reservedEscape=[SnapZyTests.Native]::RegisterHotKey([IntPtr]::Zero,9882,0x4000,27)
                if(-not $reservedEscape){throw 'Could not reserve the Escape conflict fixture.'}
                try {
                    $script:fixtureScrollOffset=0;$script:autoWheelCount=0
                    Set-ScrollFixture 0 -Fast
                    $controller=Begin-ScrollFixture $true
                    Wait-For {[SnapZyTests.Native]::Text([SnapZyTests.Native]::GetDlgItem($controller,401)).EndsWith('460 px')} 'Escape fallback did not accept its first scroll.' 15000 | Out-Null
                    [SnapZyTests.Native]::EscapeAt($fixture.Handle,$PID)
                    $preview=Wait-For {Window-Of 'SnapZyRustPreview.Capture'} 'Escape fallback failed when another process owns the hotkey.' 15000
                    $count=$script:autoWheelCount
                    Start-Sleep -Milliseconds 1200;[Windows.Forms.Application]::DoEvents()
                    if($script:autoWheelCount -ne $count){throw 'Escape-stopped job sent late wheel input.'}
                    Save-Png $preview 'auto-escape-conflict.png' | Out-Null
                    Close-Preview $preview
                    $measurements.scrolling_escape_fallback='PASS: OS Escape stops automatic capture even when another owner reserves the hotkey; no late wheel input'
                }finally{[SnapZyTests.Native]::UnregisterHotKey([IntPtr]::Zero,9882) | Out-Null}
            }
            $fixture.Remove_MouseWheel($autoHandler)
            $renderTimer.Stop();$renderTimer.Dispose()
            $fixture.BackgroundImage.Dispose();$fixture.BackgroundImage=$null
            $measurements.automatic_scrolling='PASS: product wheel messages, 900ms delayed source rendering, exact frozen-header/footer PNG, Pause/Resume, separate untitled-cover and focus-only pauses, explicit resume, no-motion reason, Stop partial, Cancel/no late input'
        }
    }
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

    $process.Refresh()
    $measurements.peak_working_set_bytes=$process.PeakWorkingSet64
    $measurements.final_gdi_objects=[SnapZyTests.Native]::GetGuiResources($process.Handle,0)
    [SnapZyTests.Native]::PostMessage($main,0x800a,[IntPtr](108),[IntPtr]::Zero) | Out-Null
    if (-not $process.WaitForExit(5000)) {throw 'The app did not exit.'}
    if ($process.ExitCode -ne 0) {throw "App exit code: $($process.ExitCode)"}
    $measurements.result='PASS'
    if($MessageOnly){$measurements.input_delivery='Posted/synchronous native messages only; physical mouse and keyboard delivery were not tested.'}
    $measurements.coverage='Native launcher EN/TH, Esc, repeat input, region/window/desktop capture, original PNG pixels, Unicode filename, hotkey conflict/success, exit'
    if ($EditorChecks) {$measurements.coverage+=', pen/line/arrow/rectangle, move/style/delete, crop, Undo/Redo, gesture cancellation, editor EN/TH, toolbar screenshot, compact bounds, clipboard paste pixels'}
    if ($HighlighterChecks) {$measurements.coverage+=', native highlighter controls/defaults, exact multiply pixels, live/final parity, opacity endpoints/Undo/Redo, color/width, move/Delete/Undo, independent Pen settings, cancellation, EN/TH Help and exact PNG/clipboard parity'}
    if ($GeometryChecks) {$measurements.coverage+=', two grouped shape popups/remembered choices/cancel/accessibility and pixels, immediate bend handle without Select, continued drawing, disabled fill stroke fields, curve bend/reset/cancel and history, dashed/dotted menu and pixels, live old-stroke erasure and upper-layer preservation, arrowhead/translation/Delete, long-image curve feedback, EN/TH Help, exact PNG/clipboard parity'}
    if ($OcrChecks) {$measurements.coverage+=', OCR result window controls, Thai + English default, no recognition on open, close lifecycle'}
    if ($EffectsChecks) {$measurements.coverage+=', Effects EN/TH panel, defaults, Apply/Cancel/Esc, invalid settings, shadow pixels and text outline, one-step history and exact PNG/clipboard parity'}
    if ($ShapeResizeChecks -and -not $SkipShapeResize) {$measurements.coverage+=', eight visible immediate handles on all four shapes, corner/side/top resize without Select, Undo/Redo, Esc cancellation, no duplicate object'}
    if ($ShapeResizeChecks) {$measurements.coverage+=', ordinary opacity 0/50/100 and history, invisible selection, exact PNG/clipboard parity and non-overlapping text/bubble opacity controls EN/TH'}
    if ($AnnotationChecks) {$measurements.coverage+=', native multiline Thai/English text, edit/cancel/Undo, font/circle/blur parameters, step resume, reversible blur, annotation PNG/clipboard parity, large-composition export gating and UI ping'}
    if ($ResizeChecks) {$measurements.coverage+=', visible text resize handles, side reflow without changing font, proportional corner font scaling, one-edit Undo/Redo, Escape/capture-loss cancellation, resized PNG/clipboard parity'}
    if ($CalloutChecks) {$measurements.coverage+=', transparent bubble default, independent fill palette/toggle memory, border/font independence and Undo field sync, visible resize/tail handles, native tail drag Undo/Redo/cancellation, multiline Thai/English bubble edit, exact PNG/clipboard parity'}
    if ($LayerChecks) {$measurements.coverage+=', four layer commands and boundaries, selection survives ordering, exact overlap/Undo/Redo output, uncommitted move cancellation, EN/TH accessible labels and Help, layer PNG/clipboard parity'}
    if ($MultiImageChecks) {$measurements.coverage+=', PNG/JPEG/BMP native file import, clipboard image paste, image move/fixed-aspect corner resize/cancel, expanded white canvas, one-edit image auto-expansion, malformed/oversized import rejection without document changes, image ordering/delete/Undo/Redo, exact composition PNG/clipboard parity'}
    if ($ScrollChecks) {$measurements.coverage+=', hover frame before click, visible non-activating selection, no scroll before confirmation, click-release start, scrolling frozen header/footer exact pixels, overlay teardown/mouse capture release/foreground handoff, non-activating controller including a test-owned SendInput click, source-window wheel messages, target-movement pause, Stop/Cancel, no late cancelled preview, scrolling PNG/clipboard parity'}
    if($MessageOnly){$measurements.coverage=$measurements.coverage.Replace('including a test-owned SendInput click','checked with native messages')}
    if ($AutoScrollChecks) {$measurements.coverage+=', product-driven targeted automatic wheel messages, default/mode persistence, exact auto PNG, Pause/Resume, untitled cover and focus-loss latched pause, no-motion completion, Cancel/no late input; not a Lark or hardware-wheel test'}
    $measurements.limitations='Clipboard DIB serialization is unit-tested; actual clipboard paste and physical key delivery require tester checks. Timings are window detection rather than completed paint.'
    if ($EditorChecks -or $AnnotationChecks -or $ResizeChecks -or $ScrollChecks -or $HighlighterChecks -or $GeometryChecks -or $ShapeResizeChecks -or $EffectsChecks) {$measurements.limitations='Clipboard was read through Windows Forms and checked pixel-by-pixel; individual chat apps and physical key delivery need tester checks. ScrollChecks uses test-owned SendInput clicks and direct source wheel messages; OS wheel delivery is a separate InputProbe and must not be inferred from a ScrollChecks pass. Native monitor DPI coverage is listed above when EditorChecks is enabled. Timings are window detection rather than completed paint.'}
    if($MessageOnly){$measurements.limitations=$measurements.limitations.Replace('test-owned SendInput clicks','native window messages instead of physical clicks')}
    if($EscapeInputChecks){
        $measurements.input_delivery='Native window messages plus guarded OS Escape key injection on a test-owned foreground window; not a hardware-key test.'
        $measurements.limitations+=' OS Escape injection is not a hardware-key test on Lark.'
    }
    $measurements | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $runDirectory 'results.json') -Encoding utf8
    Write-Output "PASS native desktop smoke checks. Evidence: $runDirectory"
    $measurements | ConvertTo-Json
} catch {
    $measurements.result='FAIL'
    $measurements.failure=$_.Exception.Message
    $measurements | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $runDirectory 'partial-results.json') -Encoding utf8
    throw
} finally {
    if ($reserved) {[SnapZyTests.Native]::UnregisterHotKey([IntPtr]::Zero,9881) | Out-Null}
    if ($process -and -not $process.HasExited) {$process.Kill();$process.WaitForExit()}
    if ($fixture) {$fixture.Close();$fixture.Dispose()}
    [SnapZyTests.Native]::RestoreCursor()
    [SnapZyTests.Native]::SetThreadDpiAwarenessContext($oldDpi) | Out-Null
}
