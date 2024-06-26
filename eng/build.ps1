# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

#Requires -PSEdition Core

<#
    .SYNOPSIS
        Build: Bootstraps psake and invokes the build.
#>
[cmdletbinding()]
param(
    [Parameter(Position = 0, Mandatory = 0)]
    [string]$buildFile = "$(Join-Path $PSScriptRoot psakefile.ps1)",
    [Parameter(Position = 1, Mandatory = 0)]
    [string[]]$taskList = @(),
    [Parameter(Position = 2, Mandatory = 0)]
    [switch]$docs = $false,
    [Parameter(Position = 3, Mandatory = 0)]
    [System.Collections.Hashtable]$parameters = @{},
    [Parameter(Position = 4, Mandatory = 0)]
    [System.Collections.Hashtable]$properties = @{},
    [Parameter(Position = 5, Mandatory = $false)]
    [switch]$detailedDocs = $false
)

# PS 7.3 introduced exec alias which breaks the build.
Remove-Item alias:exec -ErrorAction SilentlyContinue

$scriptPath = $(Split-Path -Path $MyInvocation.MyCommand.path -Parent)

# '[p]sake' is the same as 'psake' but $Error is not polluted
Remove-Module -Name [p]sake -Verbose:$false
Join-Path $scriptPath "psake" "4.9.0" "psake.psm1" | Import-Module -Force -Verbose:$false
if ($help) {
    Get-Help -Name Invoke-psake -Full
    return
}

if ($buildFile -and (-not (Test-Path -Path $buildFile))) {
    $absoluteBuildFile = (Join-Path -Path $scriptPath -ChildPath $buildFile)
    if (Test-path -Path $absoluteBuildFile) {
        $buildFile = $absoluteBuildFile
    }
}

$nologo = $true
$framework = $null
$initialization = {}
Invoke-psake $buildFile $taskList $framework $docs $parameters $properties $initialization $nologo $detailedDocs $notr

if (!$psake.build_success) {
    exit 1
}
