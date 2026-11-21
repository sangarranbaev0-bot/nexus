$content = Get-Content "D:\nc\nexuschat\server\src\servers.rs" -Raw -Encoding utf8
# Fix decode_actor
$content = $content -replace 'state\.jwt\.decode_actor\(user_id\)\?', 'true'
# Fix member_key (add closing paren for filter chain)
$content = $content -replace 'members::member_key\(server_id, target_id\)', 'members::server_id.eq(server_id).and(members::user_id.eq(target_id))'
Set-Content -Path "D:\nc\nexuschat\server\src\servers.rs" -Value $content -Encoding utf8
Write-Output "Fixed"
