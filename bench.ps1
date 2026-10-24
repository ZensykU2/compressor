param([string]$Folder = "data/silesia")

cargo build --release --quiet
$exe = ".\target\release\compressor.exe"
$tmp = "data/tmp"
New-Item -ItemType Directory -Force $tmp | Out-Null

"| File | Original | Isolated | Time (s) | gzip -9 | xz -9 | zstd -19 |"
"|---|---:|---:|---:|---:|---:|---:|"

foreach ($f in Get-ChildItem $Folder -File) {
    if ($f.Extension -ne "") { continue }

    $time = Measure-Command {
        & $exe compress $f.FullName "$tmp/x.cmp"
    }
    & $exe decompress "$tmp/x.cmp" "$tmp/x.out"

    $previous = (Get-FileHash $f.FullName).Hash
    $after = (Get-FileHash "$tmp/x.out").Hash
    if ($previous -ne $after) {
        Write-Error "ROUNDTRIP BROKEN on $($f.Name)!"
    }
    $isolated = (Get-Item "$tmp/x.cmp").Length

    Copy-Item $f.FullName "$tmp/x" -Force
    gzip -9 -k -f "$tmp/x"
    xz -9 -k -f "$tmp/x"
    zstd -19 -k -f -q "$tmp/x"

    $gz = (Get-Item "$tmp/x.gz").Length
    $xzG = (Get-Item "$tmp/x.xz").Length
    $zst = (Get-Item "$tmp/x.zst").Length

    "| $($f.Name) | $($f.Length) | $isolated | {0:N2} | $gz | $xzG | $zst |" -f $time.TotalSeconds
}

Remove-Item -Recurse -Force $tmp
