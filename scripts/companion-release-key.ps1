# Internal release tooling. Windows DPAPI protects the private XML at rest.
function Read-CompanionReleaseKey([string]$Path) {
    $document = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    if ($document.format -ne 1 -or $document.protection -ne 'windows-dpapi-current-user') { throw 'Unsupported release-key protection.' }
    $plain = $null; $rsa = [Security.Cryptography.RSA]::Create()
    try {
        $plain = [Security.Cryptography.ProtectedData]::Unprotect([Convert]::FromBase64String($document.protected_key),$null,[Security.Cryptography.DataProtectionScope]::CurrentUser)
        $rsa.FromXmlString([Text.Encoding]::UTF8.GetString($plain))
        $public = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($rsa.ToXmlString($false)))
        if ($rsa.KeySize -lt 3072 -or $public -cne $document.public_key) { throw 'Release-key public identity does not match.' }
        return $rsa
    } catch { $rsa.Dispose(); throw 'Cannot unlock the release key. Use its original Windows account and machine.' }
    finally { if ($plain) { [Array]::Clear($plain,0,$plain.Length) } }
}
function Assert-CompanionPublicKey([string]$PublicKey) {
    $rsa = [Security.Cryptography.RSA]::Create()
    try {
        $xml = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($PublicKey))
        $rsa.FromXmlString($xml)
        if ($rsa.KeySize -lt 3072 -or $xml -cne $rsa.ToXmlString($false)) { throw 'Use a canonical RSA public-only key of at least 3072 bits.' }
    } finally { $rsa.Dispose() }
}
