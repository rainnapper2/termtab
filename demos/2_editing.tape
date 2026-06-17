Output demos/2_editing.gif
Set FontSize 18
Set Width 1200
Set Height 600
Set Theme "Catppuccin Macchiato"

Type "cargo run -- demos/2_editing.json"
Enter
Sleep 1s
Type "jjjjj"
Type "R"
Type "3 5 7"
Sleep 500ms
Escape
Type "kl"
Type "R"
Type "3 5 7"
Sleep 500ms
Escape
# Turn into chromatic scale on A string (currently at col 12)
Type "hhh"
Sleep 500ms
Type ">>"
Type "r4"
Sleep 500ms
Type "llll"
Sleep 500ms
Type ">>"
Type "r6"
Sleep 500ms

# Same for E string (currently at col 14 because A string shifted it)
# Wait, A string is string 4. E string is string 5.
Type "j"
# Col 14 to Col 2 (the 5 on E string) -> 12 left
Type "hhhhhhhhhh"
Sleep 500ms
Type ">>"
Type "r4"
Sleep 500ms
# Col 2 -> Col 6 (the 7 on E string) -> 4 right
Type "llll"
Sleep 500ms
Type ">>"
Sleep 500ms
Type "r6"
Enter
Sleep 2s
Type ":wq"
Enter
