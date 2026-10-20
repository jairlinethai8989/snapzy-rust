param(
    [Parameter(Mandatory)][string]$Executable,
    [switch]$AllowClipboard,
    [switch]$RemainingChecks,
    [string]$OutputDirectory
)
$ErrorActionPreference='Stop'
if($AllowClipboard){throw 'Interactive clipboard automation is disabled. Use the isolated native clipboard test instead.'}
if ([Threading.Thread]::CurrentThread.ApartmentState -ne 'STA') { throw 'Run this test with pwsh -STA.' }
Add-Type -AssemblyName System.Windows.Forms,System.Drawing
Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
namespace SnapZyVideoTest {
 public static class Native {
  public delegate bool Callback(IntPtr h,IntPtr p);
  [StructLayout(LayoutKind.Sequential)] public struct Rect {public int L,T,R,B;}
  [StructLayout(LayoutKind.Sequential)] public struct Point {public int X,Y;}
  [DllImport("user32.dll")] public static extern bool EnumWindows(Callback f,IntPtr p);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h,Callback f,IntPtr p);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h,StringBuilder b,int n);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h,StringBuilder b,int n);
  [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr h,int id);
  [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h,uint m,IntPtr w,IntPtr l);
  [DllImport("user32.dll")] public static extern IntPtr SendMessageW(IntPtr h,uint m,IntPtr w,IntPtr l);
  [DllImport("user32.dll",CharSet=CharSet.Unicode,EntryPoint="SendMessageTimeoutW")] public static extern IntPtr ReadText(IntPtr h,uint m,IntPtr w,StringBuilder text,uint flags,uint timeout,out UIntPtr result);
  [DllImport("user32.dll",CharSet=CharSet.Unicode,EntryPoint="SendMessageTimeoutW")] public static extern IntPtr WriteText(IntPtr h,uint m,IntPtr w,string text,uint flags,uint timeout,out UIntPtr result);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h,IntPtr after,int x,int y,int w,int height,uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h,out Rect r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h,out Rect r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h,ref Point p);
  [DllImport("user32.dll")] public static extern bool ScreenToClient(IntPtr h,ref Point p);
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(Point p);
  [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h,uint flags);
  public static string Class(IntPtr h){var b=new StringBuilder(256);GetClassName(h,b,256);return b.ToString();}
  public static string Text(IntPtr h){var b=new StringBuilder(32768);UIntPtr result;ReadText(h,0xd,(IntPtr)b.Capacity,b,2,1000,out result);return b.ToString();}
  public static void SetText(IntPtr h,string text){UIntPtr result;if(WriteText(h,0xc,IntPtr.Zero,text,2,1000,out result)==IntPtr.Zero)throw new InvalidOperationException("Test control did not accept text.");}
  public static IntPtr[] Windows(int pid,string cls){var list=new List<IntPtr>();EnumWindows((h,p)=>{uint id;GetWindowThreadProcessId(h,out id);if(id==pid && Class(h)==cls)list.Add(h);return true;},IntPtr.Zero);return list.ToArray();}
  public static IntPtr[] Children(IntPtr h){var list=new List<IntPtr>();EnumChildWindows(h,(child,p)=>{list.Add(child);return true;},IntPtr.Zero);return list.ToArray();}
 }
}
'@
$Executable=(Resolve-Path -LiteralPath $Executable).Path
if(!$OutputDirectory){$OutputDirectory=Join-Path $PSScriptRoot ("..\work\video-preview-"+[guid]::NewGuid().ToString('N'))}
$run=[IO.Path]::GetFullPath($OutputDirectory)
if(Test-Path -LiteralPath $run){throw 'Use a new test output directory.'}
New-Item -ItemType Directory -Path $run | Out-Null
$process=$null
$form=$null
$timer=$null
$checks=[Collections.Generic.List[string]]::new()
function Wait-For([scriptblock]$Condition,[string]$Failure,[int]$Seconds=20){
 $clock=[Diagnostics.Stopwatch]::StartNew()
 do{
  [Windows.Forms.Application]::DoEvents()
  if($process -and $process.HasExited){throw "Application exited: $($process.ExitCode). $Failure"}
  $result=& $Condition
  if($result){return $result}
  Start-Sleep -Milliseconds 50
 }while($clock.Elapsed.TotalSeconds -lt $Seconds)
 throw $Failure
}
function Windows-Of([string]$Class){ @([SnapZyVideoTest.Native]::Windows($process.Id,$Class)) }
function Command([IntPtr]$Window,[int]$Id){[SnapZyVideoTest.Native]::PostMessageW($Window,0x111,[IntPtr]$Id,[IntPtr]::Zero)|Out-Null}
function Status([IntPtr]$Window){[SnapZyVideoTest.Native]::Text([SnapZyVideoTest.Native]::GetDlgItem($Window,401))}
function Dialog { @((Windows-Of '#32770') | Where-Object {[SnapZyVideoTest.Native]::IsWindowVisible($_)}) | Select-Object -Last 1 }
function Choose-Task([IntPtr]$Dialog,[int]$Id){[SnapZyVideoTest.Native]::PostMessageW($Dialog,0x466,[IntPtr]$Id,[IntPtr]::Zero)|Out-Null}
function Preview-Windows { Windows-Of 'SnapZyRustPreview.VideoPreview' }
function Wait-Dialog { Wait-For {Dialog} 'Expected a dialog owned by the test app.' }
function Assert-Layout([IntPtr]$Preview){
 $client=[SnapZyVideoTest.Native+Rect]::new()
 [SnapZyVideoTest.Native]::GetClientRect($Preview,[ref]$client)|Out-Null
 $origin=[SnapZyVideoTest.Native+Point]::new()
 [SnapZyVideoTest.Native]::ClientToScreen($Preview,[ref]$origin)|Out-Null
 foreach($id in 1001..1010){
  $control=[SnapZyVideoTest.Native]::GetDlgItem($Preview,$id)
  if($control -eq [IntPtr]::Zero){throw "Missing preview control $id"}
  $r=[SnapZyVideoTest.Native+Rect]::new()
  [SnapZyVideoTest.Native]::GetWindowRect($control,[ref]$r)|Out-Null
  if($r.L -lt $origin.X -or $r.T -lt $origin.Y -or $r.R -gt $origin.X+$client.R -or $r.B -gt $origin.Y+$client.B -or $r.R -le $r.L -or $r.B -le $r.T){throw "Control $id is outside the preview."}
 }
}
function Record-Clip {
 $main=(Windows-Of 'SnapZyRustPreview.Launcher')[0]
 [SnapZyVideoTest.Native]::SetWindowPos($form.Handle,[IntPtr](-1),0,0,0,0,3)|Out-Null
 $form.Refresh()
 Command $main 118
 $overlay=Wait-For {(Windows-Of 'SnapZyRustPreview.Selection')|Select-Object -First 1} 'Video selection did not open.'
 $point=[SnapZyVideoTest.Native+Point]::new()
 $point.X=120;$point.Y=90
 [SnapZyVideoTest.Native]::ClientToScreen($form.Handle,[ref]$point)|Out-Null
 [SnapZyVideoTest.Native]::ScreenToClient($overlay,[ref]$point)|Out-Null
 $packed=[IntPtr](($point.X -band 65535) -bor (($point.Y -band 65535) -shl 16))
 foreach($m in @(0x200,0x201,0x202)){[SnapZyVideoTest.Native]::PostMessageW($overlay,$m,[IntPtr]::Zero,$packed)|Out-Null}
 $panel=Wait-For {(Windows-Of 'SnapZyRustPreview.Video')|Select-Object -First 1} 'Recording panel did not open.'
 Wait-For {(Status $panel) -match '00:0[12]'} 'Countdown did not transition to recording.' | Out-Null
 $before=@(Preview-Windows)
 Command $panel 806
 $preview=Wait-For {Preview-Windows | Where-Object {$_ -notin $before} | Select-Object -First 1} 'Stop did not open a preview.'
 Wait-For {[SnapZyVideoTest.Native]::IsWindowEnabled([SnapZyVideoTest.Native]::GetDlgItem($preview,1002))} ("Playback did not become ready: "+(Status $preview)) | Out-Null
 return $preview
}
try{
 $form=[Windows.Forms.Form]::new()
 $form.Text='SnapZy generated video test fixture'
 $form.ClientSize=[Drawing.Size]::new(320,240)
 $form.StartPosition=[Windows.Forms.FormStartPosition]::Manual
 $form.Location=[Drawing.Point]::new(30,60)
 $form.BackColor=[Drawing.Color]::FromArgb(200,35,70)
 $form.TopMost=$true
 $label=[Windows.Forms.Label]::new();$label.Text='SnapZy video test';$label.AutoSize=$true;$label.ForeColor=[Drawing.Color]::White;$label.Location=[Drawing.Point]::new(10,10);$form.Controls.Add($label)
 $timer=[Windows.Forms.Timer]::new();$timer.Interval=150
 $timer.add_Tick({$label.Text='SnapZy '+[DateTime]::Now.ToString('HH:mm:ss.fff')})
 $timer.Start();$form.Show()
 $process=Start-Process -FilePath $Executable -ArgumentList @('--test-data-dir',"`"$run`"") -WindowStyle Hidden -RedirectStandardError (Join-Path $run 'stderr.log') -PassThru
 Wait-For {(Windows-Of 'SnapZyRustPreview.Launcher').Count -eq 1} 'Test launcher did not open.' | Out-Null
 $preview=Record-Clip
 $checks.Add('Actual record -> Stop -> preview; no automatic Save dialog')
 if(Dialog){throw 'Save dialog unexpectedly opened after Stop.'}
 Assert-Layout $preview
 if(!$RemainingChecks){
 [SnapZyVideoTest.Native]::SetWindowPos($preview,[IntPtr](-1),380,60,760,580,0)|Out-Null
 $surface=[SnapZyVideoTest.Native]::GetDlgItem($preview,1001)
 $rect=[SnapZyVideoTest.Native+Rect]::new()
 [SnapZyVideoTest.Native]::GetWindowRect($surface,[ref]$rect)|Out-Null
 $probe=[SnapZyVideoTest.Native+Point]::new();$probe.X=[int](($rect.L+$rect.R)/2);$probe.Y=[int](($rect.T+$rect.B)/2)
 if([SnapZyVideoTest.Native]::GetAncestor([SnapZyVideoTest.Native]::WindowFromPoint($probe),2) -ne $preview){throw 'Test preview is obscured; screenshot not taken.'}
 Start-Sleep -Milliseconds 250
 $bitmap=[Drawing.Bitmap]::new($rect.R-$rect.L,$rect.B-$rect.T)
 $graphics=[Drawing.Graphics]::FromImage($bitmap)
 try{$graphics.CopyFromScreen($rect.L,$rect.T,0,0,$bitmap.Size)}finally{$graphics.Dispose()}
 try{
  $bitmap.Save((Join-Path $run 'paused-first-frame.png'))
  $pixel=$bitmap.GetPixel([int]($bitmap.Width/2),[int]($bitmap.Height/2))
  if($pixel.R -lt 140 -or $pixel.G -gt 110 -or $pixel.B -gt 130){throw "Paused preview is blank or wrong: $pixel"}
 }finally{$bitmap.Dispose()}
 $checks.Add('Paused preview contains actual recorded fixture pixels')
 $timeControl=[SnapZyVideoTest.Native]::GetDlgItem($preview,1004)
 Command $preview 1002
 Wait-For { [SnapZyVideoTest.Native]::Text($timeControl) -notlike '00:00 /*' } 'Playback time did not advance.' | Out-Null
 Command $preview 1002
 $checks.Add('Playback / pause')
 }
 Command $preview 1008
 $saveDialog=Wait-Dialog
 Command $saveDialog 2
 Wait-For {-not (Dialog)} 'Cancel did not close Save dialog.' | Out-Null
 if(-not [SnapZyVideoTest.Native]::IsWindow($preview)){throw 'Save Cancel closed preview.'}
 $checks.Add('Save Cancel preserves preview')
 $source=(Get-ChildItem -LiteralPath (Join-Path $run 'Recordings\Pending') -Filter '*.mp4' | Select-Object -First 1).FullName
 if(!$source){throw 'Retained clip missing.'}
 $saved=Join-Path $run 'saved-video.mp4'
 Command $preview 1008
 $saveDialog=Wait-Dialog
 $name=Wait-For {[SnapZyVideoTest.Native]::Children($saveDialog)|Where-Object {[SnapZyVideoTest.Native]::Class($_) -eq 'Edit' -and [SnapZyVideoTest.Native]::Text($_).Contains('SnapZy-')}|Select-Object -First 1} 'Cannot find Save dialog filename field.'
 [SnapZyVideoTest.Native]::SetText($name,$saved)
 if([SnapZyVideoTest.Native]::Text($name) -ne $saved){throw 'Save filename was not applied.'}
 Command $saveDialog 1
 Wait-For {(Status $preview) -eq 'MP4 saved'} 'MP4 export did not complete.' | Out-Null
 if(!(Test-Path -LiteralPath $source) -or !(Test-Path -LiteralPath $saved)){throw 'Save removed source or failed to publish.'}
 if((Get-FileHash -LiteralPath $source).Hash -ne (Get-FileHash -LiteralPath $saved).Hash){throw 'Save changed video bytes.'}
 $checks.Add('Export preserves retained source and exact bytes')
 $main=(Windows-Of 'SnapZyRustPreview.Launcher')[0]
 Command $main 104
 Wait-For {[SnapZyVideoTest.Native]::Text($preview).Contains('พรีวิว')} 'Language command failed.' | Out-Null
 Assert-Layout $preview
 Command $main 104
 Wait-For {[SnapZyVideoTest.Native]::Text($preview).Contains('Video preview')} 'English language command failed.' | Out-Null
 [SnapZyVideoTest.Native]::SetWindowPos($preview,[IntPtr]::Zero,0,0,640,460,6)|Out-Null
 Start-Sleep -Milliseconds 200
 Assert-Layout $preview
 $checks.Add('EN/TH refresh and minimum-size control bounds')
 [SnapZyVideoTest.Native]::PostMessageW($preview,0x10,[IntPtr]::Zero,[IntPtr]::Zero)|Out-Null
 Wait-For {-not [SnapZyVideoTest.Native]::IsWindow($preview)} 'Saved preview did not close.' | Out-Null
 $first=Record-Clip
 $second=Record-Clip
 if(@(Preview-Windows).Count -ne 2){throw 'Second recording did not preserve first preview.'}
 Command $main 108
 $review=Wait-Dialog
 Choose-Task $review 7
 $next=Wait-For { $d=Dialog;if($d -and $d -ne $review){$d} } 'Second preview review missing.'
 Choose-Task $next 2
 Wait-For {-not (Dialog)} 'Exit Cancel did not dismiss dialog.' | Out-Null
 if(@(Preview-Windows).Count -ne 2){throw 'Cancel partially closed the preview set.'}
 $checks.Add('Exit Keep Local then Cancel preserves both previews')
 foreach($window in @($first,$second)){
  [SnapZyVideoTest.Native]::PostMessageW($window,0x10,[IntPtr]::Zero,[IntPtr]::Zero)|Out-Null
  $review=Wait-Dialog
  Choose-Task $review 7
  Wait-For {-not [SnapZyVideoTest.Native]::IsWindow($window)} 'Keep Local did not close preview.' | Out-Null
 }
 Command $main 108
 $process.WaitForExit(10000)|Out-Null
 if(!$process.HasExited -or $process.ExitCode -ne 0){throw 'Test application did not exit cleanly.'}
 $checks.Add('Keep Local and app exit retain completed files')
 $result=[pscustomobject]@{Checks=$checks.ToArray();Output=$run;ClipboardChecked=$false}
 $result|ConvertTo-Json -Depth 4|Set-Content -LiteralPath (Join-Path $run 'result.json')
 $result|ConvertTo-Json -Depth 4
}catch{
 [pscustomobject]@{Error=$_.ToString();Stack=$_.ScriptStackTrace;Checks=$checks.ToArray()}|ConvertTo-Json -Depth 4|Set-Content -LiteralPath (Join-Path $run 'failure.json')
 throw
}finally{
 if($timer){$timer.Stop();$timer.Dispose()}
 if($form){$form.Close();$form.Dispose()}
 if($process -and !$process.HasExited){$process.Kill();$process.WaitForExit()}
}
