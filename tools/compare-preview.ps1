param(
    [string]$Baseline = (Join-Path $PSScriptRoot '..\artifacts\baseline-alpha1\snapzy-rust.exe'),
    [string]$Candidate = (Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-msvc\release\snapzy-rust.exe'),
    [ValidateRange(3,20)][int]$Samples = 3
)
$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot
$directory=Join-Path $root ('artifacts\comparison-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $directory -Force | Out-Null
$records=@()
for($n=0;$n -lt $Samples;$n++) {
    $order=if($n%2 -eq 0){@('baseline','candidate')}else{@('candidate','baseline')}
    foreach($name in $order) {
        $exe=if($name -eq 'baseline'){$Baseline}else{$Candidate}
        $exe=(Resolve-Path -LiteralPath $exe).Path
        $output=& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'smoke-test.ps1') -Executable $exe
        if($LASTEXITCODE -ne 0){throw "Smoke comparison failed: $name sample $n"}
        $line=$output | Where-Object {$_ -match '^PASS native desktop smoke checks\. Evidence: (.+)$'} | Select-Object -Last 1
        if(-not $line){throw 'Comparison evidence path missing.'}
        $path=$line.Substring($line.IndexOf('Evidence: ')+10).Trim()
        $result=Get-Content -LiteralPath (Join-Path $path 'results.json') -Raw | ConvertFrom-Json
        $log=Get-Content (Join-Path $path 'Logs\performance-*.log')
        foreach($stage in @('launcher.ready','capture.selection-first-paint','capture.preview-first-paint')) {
            $pattern=' '+[regex]::Escape($stage)+' ([0-9.]+)ms$'
            $entry=$log | Where-Object {$_ -match $pattern} | Select-Object -First 1
            if($entry -and $entry -match $pattern) {$result | Add-Member -NotePropertyName ($stage.Replace('.','_').Replace('-','_')+'_ms') -NotePropertyValue ([double]$Matches[1])}
        }
        $records+=[pscustomobject]@{build=$name;sample=$n+1;version=(Get-Item -LiteralPath $exe).VersionInfo.ProductVersion;sha256=(Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash;evidence=$path;result=$result}
        Write-Output "$name sample $($n+1): launch $($result.launcher_window_ms)ms; selection $($result.area_selection_window_ms)ms; preview $($result.area_preview_window_ms)ms"
    }
}
$summary=@()
foreach($name in @('baseline','candidate')) {
    $group=@($records | Where-Object {$_.build -eq $name})
    foreach($metric in @('launcher_window_ms','area_selection_window_ms','area_preview_window_ms','launcher_ready_ms','capture_selection_first_paint_ms','capture_preview_first_paint_ms','peak_working_set_bytes')) {
        $values=@($group | ForEach-Object {$_.result.$metric} | Sort-Object)
        $middle=[int][Math]::Floor($values.Count/2)
        $median=if($values.Count%2){$values[$middle]}else{($values[$middle-1]+$values[$middle])/2}
        $summary+=[pscustomobject]@{build=$name;metric=$metric;samples=$values.Count;median=$median;slowest=$values[-1]}
    }
}
$report=[ordered]@{
    date=(Get-Date).ToString('yyyy-MM-dd',[Globalization.CultureInfo]::InvariantCulture);timezone='Asia/Bangkok';os=[Environment]::OSVersion.VersionString;
    scope='Same-machine Release builds, alternating order, identical native smoke fixtures. Window-detection timings are not completed-paint timings. Baseline/Candidate versions and hashes are recorded below; neither is a .NET benchmark. No editor gestures in comparable samples.';
    samples=$records;summary=$summary
}
$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $directory 'comparison.json') -Encoding utf8
$summary | Format-Table -AutoSize | Out-String | Write-Output
Write-Output "Evidence: $directory"
