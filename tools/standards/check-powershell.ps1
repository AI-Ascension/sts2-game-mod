# Parse PowerShell sources without executing operator/game actions.
param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'

function Assert-ParsedFile([string]$Path) {
    $tokens = $null
    $parseErrors = $null
    [void][System.Management.Automation.Language.Parser]::ParseFile(
        $Path, [ref]$tokens, [ref]$parseErrors)
    if ($parseErrors.Count -ne 0) {
        throw "PowerShell parse rejected: $Path"
    }
}

if ($SelfTest) {
    $fixture = [System.IO.Path]::GetTempFileName()
    try {
        [System.IO.File]::WriteAllText($fixture, 'if ($true) {')
        $rejected = $false
        try { Assert-ParsedFile $fixture } catch {
            if ($_.Exception.Message -notlike 'PowerShell parse rejected:*') { throw }
            $rejected = $true
        }
        if (-not $rejected) { throw 'Malformed PowerShell passed parser gate' }
        [System.IO.File]::WriteAllText($fixture, 'Write-Output "synthetic data only"')
        Assert-ParsedFile $fixture
        Write-Output 'PowerShell parser: positive and malformed-source fixtures passed'
    } finally {
        [System.IO.File]::Delete($fixture)
    }
    exit 0
}

$repository = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$files = @(& git -C $repository -c core.quotepath=false ls-files -- '*.ps1')
if ($LASTEXITCODE -ne 0) { throw 'Git source enumeration failed' }
if ($files.Count -eq 0) { throw 'No PowerShell sources found' }
foreach ($file in $files) {
    $source = Join-Path $repository $file
    $item = Get-Item -LiteralPath $source
    if ($item.LinkType) { throw 'Linked PowerShell source is not an admitted target' }
    Assert-ParsedFile $source
}
Write-Output "PowerShell parser checked $($files.Count) tracked sources without execution"
